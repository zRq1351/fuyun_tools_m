//! 浮云剪贴板移动端（Tauri v2）
//!
//! 架构：Vue 前端 + Rust 核心。核心只做一件事——与 PC 端同步服务通信：
//! 探测/配对/推送（手机→电脑）、前台轮询接收（电脑→手机，自动写剪贴板）。

mod discovery;
mod sync_client;
mod sync_proto;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sync_client::SyncClient;
use sync_proto::HistoryItem;
use sync_proto::SyncMessage;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// 前台轮询间隔（秒）。v1 仅前台同步，后台推送后续版本再做。
const POLL_INTERVAL_SECS: u64 = 3;

/// 同步循环状态（进程内单例）。running 用 Arc 以便任务闭包持有独立引用。
struct SyncLoop {
    running: Arc<AtomicBool>,
    task: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
}

/// 推送到前端的事件载荷（与 src/App.vue 约定）
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncItemEvent {
    id: i64,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

fn loop_state(app: &AppHandle) -> &SyncLoop {
    app.state::<SyncLoop>().inner()
}

// ---------- 命令 ----------

/// 探测电脑连接（免认证）
#[tauri::command]
async fn sync_probe_status(host: String, port: u16) -> Result<serde_json::Value, String> {
    let client = SyncClient::new();
    let status = client.status(&host, port).await?;
    Ok(serde_json::json!({
        "pcName": status.pc_name,
        "paired": status.paired,
    }))
}

/// 配对：配对码换令牌
#[tauri::command]
async fn sync_pair(host: String, port: u16, code: String) -> Result<serde_json::Value, String> {
    let client = SyncClient::new();
    let result = client.pair(&host, port, code.trim()).await?;
    Ok(serde_json::json!({
        "token": result.token,
        "pcName": result.pc_name,
    }))
}

/// 发送文本到电脑
#[tauri::command]
async fn sync_push_text(
    host: String,
    port: u16,
    token: String,
    text: String,
) -> Result<(), String> {
    let client = SyncClient::new();
    client.push_text(&host, port, &token, &text).await
}

/// 发送图片（base64）到电脑
#[tauri::command]
async fn sync_push_image(
    host: String,
    port: u16,
    token: String,
    image_base64: String,
) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| format!("图片数据解码失败：{}", e))?;
    let client = SyncClient::new();
    client.push_image_bytes(&host, port, &token, &bytes).await
}

/// 启动前台轮询循环（幂等；重复调用先停旧的）
#[tauri::command]
async fn sync_start_loop(
    app: AppHandle,
    host: String,
    port: u16,
    token: String,
) -> Result<(), String> {
    sync_stop_loop_inner(&app);
    // 克隆 Arc 引用，避免 app 移入闭包时与借用冲突
    let running = loop_state(&app).running.clone();
    let task_slot = loop_state(&app).task.clone();
    running.store(true, Ordering::SeqCst);

    let handle = tauri::async_runtime::spawn(async move {
        let client = SyncClient::new();
        let mut last_id: i64 = 0;
        let mut failures = 0u32;
        while running.load(Ordering::SeqCst) {
            match client.history(&host, port, &token, last_id).await {
                Ok(items) => {
                    failures = 0;
                    for item in items {
                        if item.id > last_id {
                            last_id = item.id;
                        }
                        process_item(&app, &client, &host, port, &token, &item).await;
                    }
                }
                Err(e) => {
                    failures += 1;
                    log::warn!("同步轮询失败（第 {} 次）：{}", failures, e);
                    // 断线时退避：3s → 6s → 12s … 上限 30s
                    let backoff =
                        Duration::from_secs((POLL_INTERVAL_SECS << failures.min(3)).min(30));
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            }
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
        log::info!("同步轮询循环已退出");
    });
    task_slot.lock().unwrap().replace(handle);
    Ok(())
}

/// 停止前台轮询循环
#[tauri::command]
async fn sync_stop_loop(app: AppHandle) -> Result<(), String> {
    sync_stop_loop_inner(&app);
    Ok(())
}

/// 远程开启电脑端投屏（手机 App「投屏到电脑」入口）
#[tauri::command]
async fn sync_mirror_start(
    host: String,
    port: u16,
    token: String,
) -> Result<serde_json::Value, String> {
    let client = SyncClient::new();
    client.mirror_start(&host, port, &token).await
}

/// 自动发现局域网内的电脑（mDNS 优先，子网扫描兜底）
#[tauri::command]
async fn sync_discover(timeout_ms: Option<u64>) -> Result<serde_json::Value, String> {
    let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(2000));
    let list = discovery::discover(timeout).await;
    Ok(serde_json::json!({ "computers": list }))
}

/// 查询电脑端投屏会话状态
#[tauri::command]
async fn sync_mirror_status(
    host: String,
    port: u16,
    token: String,
) -> Result<serde_json::Value, String> {
    let client = SyncClient::new();
    client.mirror_status(&host, port, &token).await
}

fn sync_stop_loop_inner(app: &AppHandle) {
    let state = loop_state(app);
    state.running.store(false, Ordering::SeqCst);
    if let Some(handle) = state.task.lock().unwrap().take() {
        handle.abort();
    }
}

/// 处理一条收到内容：写剪贴板 + 通知前端
async fn process_item(
    app: &AppHandle,
    client: &SyncClient,
    host: &str,
    port: u16,
    token: &str,
    item: &HistoryItem,
) {
    let event = match &item.message {
        SyncMessage::Text { text } => {
            if let Err(e) = app.clipboard().write_text(text) {
                log::warn!("写入剪贴板失败：{}", e);
            }
            SyncItemEvent {
                id: item.id,
                kind: "text".into(),
                text: Some(text.clone()),
                data_url: None,
                width: None,
                height: None,
            }
        }
        SyncMessage::Image { width, height, .. } => {
            match client.download_image(host, port, token, item.id).await {
                Ok(bytes) => {
                    let data_url = format!(
                        "data:{};base64,{}",
                        sniff_mime(&bytes),
                        base64_encode(&bytes)
                    );
                    if let Err(e) = write_image_to_clipboard(app, &bytes) {
                        log::warn!("写入图片剪贴板失败：{}", e);
                    }
                    SyncItemEvent {
                        id: item.id,
                        kind: "image".into(),
                        text: None,
                        data_url: Some(data_url),
                        width: Some(*width),
                        height: Some(*height),
                    }
                }
                Err(e) => {
                    log::warn!("下载同步图片失败：{}", e);
                    return;
                }
            }
        }
    };
    let _ = app.emit("sync-item", event);
}

/// 按历史条目 id 重新下载图片并写入剪贴板
#[tauri::command]
async fn sync_write_image_clipboard(
    app: AppHandle,
    host: String,
    port: u16,
    token: String,
    id: i64,
) -> Result<(), String> {
    let client = SyncClient::new();
    let bytes = client.download_image(&host, port, &token, id).await?;
    write_image_to_clipboard(&app, &bytes)
}

/// 读取本机剪贴板文本
#[tauri::command]
async fn clipboard_read_text(app: AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| format!("读取剪贴板失败：{}", e))
}

/// 写文本到本机剪贴板
#[tauri::command]
async fn clipboard_write_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("写入剪贴板失败：{}", e))
}

// ---------- 工具 ----------

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 根据魔数嗅探图片 MIME（PC 端默认存 webp）
fn sniff_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else {
        "image/png"
    }
}

/// 解码图片字节并写入剪贴板（PNG/JPEG/WebP/GIF/BMP）
fn write_image_to_clipboard(app: &AppHandle, bytes: &[u8]) -> Result<(), String> {
    let decoded = image::load_from_memory(bytes).map_err(|e| format!("图片解码失败：{}", e))?;
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    let tauri_image = tauri::image::Image::new_owned(rgba.into_raw(), width, height);
    app.clipboard()
        .write_image(&tauri_image)
        .map_err(|e| format!("写入图片剪贴板失败：{}", e))
}

// ---------- 应用入口 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(SyncLoop {
            running: Arc::new(AtomicBool::new(false)),
            task: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            sync_probe_status,
            sync_pair,
            sync_push_text,
            sync_push_image,
            sync_start_loop,
            sync_stop_loop,
            sync_discover,
            sync_mirror_start,
            sync_mirror_status,
            sync_write_image_clipboard,
            clipboard_read_text,
            clipboard_write_text,
        ])
        .run(tauri::generate_context!())
        .expect("启动浮云剪贴板失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniff_mime_by_magic_bytes() {
        assert_eq!(sniff_mime(&[0x89, b'P', b'N', b'G', 0, 0]), "image/png");
        assert_eq!(sniff_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(sniff_mime(b"GIF89a"), "image/gif");
        // webp: RIFF....ftyp
        let webp = [0x52u8, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50];
        assert_eq!(sniff_mime(&webp), "image/webp");
        // 未知格式回退 png
        assert_eq!(sniff_mime(&[0x00, 0x01, 0x02]), "image/png");
    }

    #[test]
    fn test_sync_message_serialization_matches_protocol() {
        // 与桌面端 server.rs 的 ClipRequest JSON 形状一致（kind 字段判别）
        let text = sync_proto::ClipRequest::Text { text: "hi".into() };
        let json = serde_json::to_string(&text).unwrap();
        assert_eq!(json, r#"{"kind":"text","text":"hi"}"#);
        let img = sync_proto::ClipRequest::Image {
            image_base64: "AAA".into(),
        };
        let json = serde_json::to_string(&img).unwrap();
        assert_eq!(json, r#"{"kind":"image","image_base64":"AAA"}"#);
    }
}
