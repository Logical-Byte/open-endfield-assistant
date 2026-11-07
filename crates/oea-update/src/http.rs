use crate as update;
use crate::error::Reason;
use std::time::Duration;

/// 构造更新请求共用的客户端配置。
fn base_client_builder(user_agent: &str) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        .redirect(reqwest::redirect::Policy::limited(10))
}

/// 构造更新服务可见的 User-Agent。
pub(super) fn update_user_agent(app_version: &str) -> String {
    // Windows 10 和 11 都使用 NT 10.0；项目不支持更早版本的 Windows。
    format!("OEA/{app_version} (Windows NT 10.0; Win64; x64)")
}

/// 使用宿主已经解析的代理地址构建更新 HTTP 客户端，`None` 表示直连。
pub(super) fn build_client(
    proxy_url: Option<&str>,
    user_agent: &str,
) -> Result<reqwest::Client, update::Error> {
    let builder = base_client_builder(user_agent);
    let builder = if let Some(url) = proxy_url {
        let proxy = reqwest::Proxy::all(url).map_err(|error| {
            update::Error::failed(
                Reason::ProxyConfiguration,
                anyhow::Error::new(error).context("代理配置失败"),
            )
        })?;
        builder.proxy(proxy)
    } else {
        builder.no_proxy()
    };

    builder.build().map_err(|error| {
        update::Error::failed(
            Reason::Network,
            anyhow::Error::new(error).context("创建 HTTP 客户端失败"),
        )
    })
}
