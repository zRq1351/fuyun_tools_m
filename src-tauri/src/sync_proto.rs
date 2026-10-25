//! 与 PC 端（fuyun_tools `features/sync`）一致的同步协议类型。
//!
//! ⚠️ 协议变更必须双边同步：桌面端 `src-tauri/src/features/sync/` 与本文件。
//! 协议文档见 fuyun_tools/docs/mobile-sync.md。

use serde::{Deserialize, Serialize};

/// 一条同步内容（对应 PC 端 SyncMessage）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncMessage {
    Text {
        text: String,
    },
    Image {
        /// PC 侧图片文件绝对路径（本机不直接使用，经 /api/image 下载）
        path: String,
        width: u32,
        height: u32,
    },
}

/// 带序号的历史条目（PC 端 /api/history 返回元素）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: i64,
    #[serde(flatten)]
    pub message: SyncMessage,
}

/// 配对请求（POST /api/pair）
#[derive(Serialize)]
pub struct PairRequest {
    pub code: String,
}

/// 配对响应（POST /api/pair 200）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponse {
    pub token: String,
    #[serde(alias = "pcName")]
    pub pc_name: String,
}

/// 状态响应（GET /api/status）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    #[serde(alias = "pcName")]
    pub pc_name: String,
    pub paired: bool,
}

/// 推送到 PC 的请求体（POST /api/clip）
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClipRequest {
    Text { text: String },
    Image { image_base64: String },
}
