//! 局域网自动发现电脑：优先 mDNS 浏览 `_fuyun._tcp.local.`，
//! 多播被路由器/省电策略拦截时回退为 /24 子网扫描（探测 /api/status）。

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 与桌面端同步服务保持一致的默认端口（子网扫描兜底时使用）
const DEFAULT_PORT: u16 = 45817;
const SERVICE_TYPE: &str = "_fuyun._tcp.local.";

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredPc {
    pub name: String,
    pub host: String,
    pub port: u16,
}

/// 自动发现电脑：mDNS 优先，空结果回退子网扫描。
pub async fn discover(timeout: Duration) -> Vec<DiscoveredPc> {
    let mdns_timeout = timeout.min(Duration::from_millis(2500));
    let found = tauri::async_runtime::spawn_blocking(move || browse_mdns(mdns_timeout))
        .await
        .unwrap_or_default();
    if !found.is_empty() {
        log::info!("mDNS 发现 {} 台电脑", found.len());
        return found;
    }
    log::info!("mDNS 未发现电脑，回退子网扫描");
    scan_subnet().await
}

/// 浏览 mDNS 服务，收集解析到的电脑列表。
fn browse_mdns(timeout: Duration) -> Vec<DiscoveredPc> {
    use mdns_sd::{ServiceDaemon, ServiceEvent};

    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            log::warn!("mDNS 守护进程启动失败: {}", e);
            return Vec::new();
        }
    };
    let receiver = match daemon.browse(SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("mDNS 浏览失败: {}", e);
            let _ = daemon.shutdown();
            return Vec::new();
        }
    };

    let deadline = Instant::now() + timeout;
    let mut out: Vec<DiscoveredPc> = Vec::new();
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(300))) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let port = info.get_port();
                let name = info
                    .get_property_val_str("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| info.get_fullname().to_string());
                // 优先 IPv4（手机侧绝大多数为 IPv4 局域网）
                let ip = info
                    .get_addresses()
                    .iter()
                    .find(|a| a.is_ipv4())
                    .or_else(|| info.get_addresses().iter().next())
                    .cloned();
                if let Some(ip) = ip {
                    let host = ip.to_string();
                    if !out.iter().any(|d| d.host == host && d.port == port) {
                        out.push(DiscoveredPc { name, host, port });
                    }
                }
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }
    let _ = daemon.shutdown();
    out
}

/// /24 子网扫描兜底：并发探测 `http://<ip>:45817/api/status`。
async fn scan_subnet() -> Vec<DiscoveredPc> {
    let Some(local) = lan_ipv4() else {
        return Vec::new();
    };
    let octets = local.octets();
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()
    {
        Ok(c) => Arc::new(c),
        Err(_) => return Vec::new(),
    };

    let mut found = Vec::new();
    // 分批并发（每批 40 个，单请求 400ms 超时），避免一次性 254 个连接拖垮手机
    for batch_start in (1u16..=254).step_by(40) {
        let mut tasks = Vec::new();
        for last in batch_start..=(batch_start + 39).min(254) {
            let last = last as u8;
            let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], last);
            if ip == local {
                continue;
            }
            let host = ip.to_string();
            let client = Arc::clone(&client);
            tasks.push(tokio::spawn(async move { probe_host(&client, host).await }));
        }
        for task in tasks {
            if let Ok(Some(pc)) = task.await {
                found.push(pc);
            }
        }
    }
    log::info!("子网扫描发现 {} 台电脑", found.len());
    found
}

/// 探测单个 IP 是否是浮云电脑端。
async fn probe_host(client: &reqwest::Client, host: String) -> Option<DiscoveredPc> {
    let url = format!("http://{}:{}/api/status", host, DEFAULT_PORT);
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let name = json.get("pcName")?.as_str()?.to_string();
    Some(DiscoveredPc {
        name,
        host,
        port: DEFAULT_PORT,
    })
}

/// 取本机局域网 IPv4：向外部地址连 UDP（不实际发包），读回本地绑定地址。
fn lan_ipv4() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(v4) => Some(v4),
        IpAddr::V6(_) => None,
    }
}
