//! 同步客户端：与 PC 端同步服务的 HTTP 通信。
//!
//! 对应 PC 端 `features/sync/server.rs` 的六个端点中的四个
//! （status / pair / clip / history / image；WS 推送暂未用，v1 采用轮询）。

use super::sync_proto::*;
use base64::Engine;

/// 展开 reqwest 错误的完整 source 链。
///
/// reqwest 的 Display 只显示顶层消息（如 "error sending request for url"），
/// 底层真实原因（connect timeout / connection refused / network unreachable /
/// DNS 失败等）藏在 `e.source()` 链里。局域网连接排障时必须展开，
/// 否则用户只看到一句没有信息量的「无法连接电脑」。
fn describe_net_error(e: &reqwest::Error) -> String {
    let mut msg = e.to_string();
    let mut src = std::error::Error::source(&e);
    while let Some(s) = src {
        msg.push_str(&format!(" | 底层原因: {}", s));
        src = s.source();
    }
    msg
}

pub struct SyncClient {
    http: reqwest::Client,
}

impl SyncClient {
    pub fn new() -> Self {
        // 局域网服务，短超时；图片上传放宽写超时
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    fn base(host: &str, port: u16) -> String {
        format!("http://{}:{}", host, port)
    }

    /// 查询 PC 状态（免认证）
    pub async fn status(&self, host: &str, port: u16) -> Result<StatusResponse, String> {
        let resp = self
            .http
            .get(format!("{}/api/status", Self::base(host, port)))
            .send()
            .await
            .map_err(|e| format!("无法连接电脑：{}", describe_net_error(&e)))?;
        if !resp.status().is_success() {
            return Err(format!("连接失败（HTTP {}）", resp.status().as_u16()));
        }
        resp.json::<StatusResponse>()
            .await
            .map_err(|e| format!("响应解析失败：{}", describe_net_error(&e)))
    }

    /// 配对：配对码换设备令牌
    pub async fn pair(&self, host: &str, port: u16, code: &str) -> Result<PairResponse, String> {
        let body = PairRequest {
            code: code.to_string(),
        };
        let resp = self
            .http
            .post(format!("{}/api/pair", Self::base(host, port)))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("无法连接电脑：{}", describe_net_error(&e)))?;
        match resp.status().as_u16() {
            403 => Err("配对码无效或已过期".to_string()),
            200 => resp
                .json::<PairResponse>()
                .await
                .map_err(|e| format!("响应解析失败：{}", describe_net_error(&e))),
            code => Err(format!("配对失败（HTTP {}）", code)),
        }
    }

    /// 发送文本到 PC 剪贴板
    pub async fn push_text(
        &self,
        host: &str,
        port: u16,
        token: &str,
        text: &str,
    ) -> Result<(), String> {
        self.push_clip(
            host,
            port,
            token,
            &ClipRequest::Text {
                text: text.to_string(),
            },
        )
        .await
    }

    /// 发送图片（原始字节，内部转 base64）到 PC 剪贴板
    pub async fn push_image_bytes(
        &self,
        host: &str,
        port: u16,
        token: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        self.push_clip(
            host,
            port,
            token,
            &ClipRequest::Image {
                image_base64: encoded,
            },
        )
        .await
    }

    async fn push_clip(
        &self,
        host: &str,
        port: u16,
        token: &str,
        body: &ClipRequest,
    ) -> Result<(), String> {
        let resp = self
            .http
            .post(format!("{}/api/clip", Self::base(host, port)))
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await
            .map_err(|e| format!("发送失败：{}", describe_net_error(&e)))?;
        match resp.status().as_u16() {
            200 => Ok(()),
            401 => Err("认证失败，请重新配对".to_string()),
            413 => Err("图片过大，超过电脑端 32MB 接收上限".to_string()),
            code => Err(format!("发送失败（HTTP {}）", code)),
        }
    }

    /// 拉取 since 之后的新历史条目
    pub async fn history(
        &self,
        host: &str,
        port: u16,
        token: &str,
        since: i64,
    ) -> Result<Vec<HistoryItem>, String> {
        let resp = self
            .http
            .get(format!(
                "{}/api/history?since={}",
                Self::base(host, port),
                since
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("拉取失败：{}", describe_net_error(&e)))?;
        match resp.status().as_u16() {
            200 => resp
                .json::<Vec<HistoryItem>>()
                .await
                .map_err(|e| format!("响应解析失败：{}", describe_net_error(&e))),
            401 => Err("认证失败，请重新配对".to_string()),
            code => Err(format!("拉取失败（HTTP {}）", code)),
        }
    }

    /// 按历史条目 id 下载图片二进制
    pub async fn download_image(
        &self,
        host: &str,
        port: u16,
        token: &str,
        id: i64,
    ) -> Result<Vec<u8>, String> {
        let resp = self
            .http
            .get(format!("{}/api/image?id={}", Self::base(host, port), id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("下载失败：{}", describe_net_error(&e)))?;
        if !resp.status().is_success() {
            return Err(format!("下载失败（HTTP {}）", resp.status().as_u16()));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("读取图片失败：{}", describe_net_error(&e)))
    }
}
