//! Windows 系统代理解析。

use ::windows::Win32::System::Registry::HKEY_CURRENT_USER;
use tracing::{info, warn};

use super::registry::{read_registry_dword, read_registry_string};

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

/// 解析 Windows 系统代理（读取注册表 `Internet Settings`）。
pub(in crate::platform) fn resolve_system_proxy() -> Result<Option<String>, String> {
    // 读取失败或值缺失一律按未启用处理（无法解析系统代理时退化为直连）。
    let enabled = read_registry_dword(HKEY_CURRENT_USER, INTERNET_SETTINGS, "ProxyEnable")
        .unwrap_or_default()
        .unwrap_or_default();
    if enabled == 0 {
        return Ok(None);
    }

    let server = read_registry_string(HKEY_CURRENT_USER, INTERNET_SETTINGS, "ProxyServer")
        .unwrap_or_default()
        .unwrap_or_default();
    let Some(proxy) = normalize_proxy_server(&server) else {
        warn!("系统代理 ProxyServer 格式无法解析: {server:?}");
        return Ok(None);
    };
    info!("解析到系统代理: {proxy}");
    Ok(Some(proxy))
}

/// 归一化注册表 `ProxyServer` 值：
/// - `host:port` → `http://host:port`
/// - `http=host:port;https=host2:port` → 优先 `https=`，否则 `http=`
/// - 仅含 `socks=` 或无法解析时返回 `None`（reqwest 未启用 socks 特性）
fn normalize_proxy_server(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.contains('=') {
        let mut chosen: Option<&str> = None;
        for part in raw.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let Some((scheme, addr)) = part.split_once('=') else {
                continue;
            };
            let addr = addr.trim();
            if addr.is_empty() {
                continue;
            }
            if matches!(scheme.trim(), "http" | "https") {
                chosen = Some(addr);
                if scheme.trim() == "https" {
                    break;
                }
            }
        }
        return chosen.map(|addr| format!("http://{addr}"));
    }

    Some(format!("http://{raw}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_proxy_server() {
        assert_eq!(
            normalize_proxy_server("127.0.0.1:7890"),
            Some("http://127.0.0.1:7890".to_string())
        );
        assert_eq!(
            normalize_proxy_server("http=127.0.0.1:7890;https=127.0.0.1:7891"),
            Some("http://127.0.0.1:7891".to_string())
        );
        assert_eq!(normalize_proxy_server("socks=127.0.0.1:1080"), None);
        assert_eq!(normalize_proxy_server(""), None);
    }
}
