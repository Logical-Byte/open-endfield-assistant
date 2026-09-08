//! 系统代理的平台接口。

/// 解析当前系统配置的代理服务器。
///
/// 返回可直接用于 HTTP 客户端的 `http://host:port`；系统代理未启用或无法解析时返回
/// `Ok(None)`。macOS 开发外壳不解析系统代理。
pub fn resolve_system_proxy() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        super::windows::proxy::resolve_system_proxy()
    }

    #[cfg(target_os = "macos")]
    {
        Ok(None)
    }
}
