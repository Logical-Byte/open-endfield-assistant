use std::time::Duration;

use crate::config::{OeaConfig, UpdateProxyMode};

/// 构造更新服务可见的 User-Agent。
pub(super) fn update_user_agent(app_version: &str) -> String {
    let os_version = tauri_plugin_os::version().to_string();
    let nt_version = os_version.split('.').take(2).collect::<Vec<_>>().join(".");
    format!("OEA/{app_version} (Windows NT {nt_version}; Win64; x64)")
}

/// 按一次调用的配置快照构建更新 HTTP 客户端。
pub(super) fn build_client(
    config: &OeaConfig,
    user_agent: &str,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        .redirect(reqwest::redirect::Policy::limited(10));

    match config.update_proxy_mode {
        UpdateProxyMode::System => {
            if let Some(url) = crate::platform::proxy::resolve_system_proxy()? {
                builder = builder.proxy(
                    reqwest::Proxy::all(&url)
                        .map_err(|error| format!("系统代理配置失败: {error}"))?,
                );
            } else {
                builder = builder.no_proxy();
            }
        }
        UpdateProxyMode::Custom => {
            if config.update_proxy_url.trim().is_empty() {
                builder = builder.no_proxy();
            } else {
                builder = builder.proxy(
                    reqwest::Proxy::all(config.update_proxy_url.trim())
                        .map_err(|error| format!("代理配置失败: {error}"))?,
                );
            }
        }
        UpdateProxyMode::None => builder = builder.no_proxy(),
    }

    builder
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
}

/// 构造显式直连的更新下载客户端。
pub(super) fn build_direct_client(user_agent: &str) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        .redirect(reqwest::redirect::Policy::limited(10))
        .no_proxy()
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::update_user_agent;

    #[test]
    fn user_agent_preserves_the_service_visible_shape() {
        let user_agent = update_user_agent("1.2.3");

        assert!(user_agent.starts_with("OEA/1.2.3 (Windows NT "));
        assert!(user_agent.ends_with("; Win64; x64)"));
    }
}
