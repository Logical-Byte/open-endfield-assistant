//! Windows 管理员权限接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::details;

/// 当前进程是否以管理员权限运行。
#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    details::admin::is_elevated()
}

/// macOS 开发外壳不使用 Windows 管理员权限。
#[cfg(target_os = "macos")]
pub fn is_elevated() -> bool {
    false
}

/// 以管理员权限重新启动当前应用；调用方随后应退出当前进程。
///
/// 返回 `Ok` 表示新进程已成功启动；用户取消 UAC 时返回 `Err`。
#[cfg(target_os = "windows")]
pub fn restart_as_admin() -> Result<()> {
    details::admin::restart_as_admin()
}

/// macOS 开发外壳不支持 Windows 提权重启。
#[cfg(target_os = "macos")]
pub fn restart_as_admin() -> Result<()> {
    Err(super::unsupported("restart_as_admin"))
}

/// 启动时自动请求管理员权限（仅 release 生效）。
///
/// 非管理员时自提权重启并退出当前进程；用户取消 UAC 时继续以普通权限运行。
/// debug 构建不处理，方便 `tauri dev` 在普通终端调试。
#[cfg(target_os = "windows")]
pub fn elevate_at_startup() {
    if !cfg!(debug_assertions) && !is_elevated() && restart_as_admin().is_ok() {
        std::process::exit(0);
    }
}

/// macOS 开发外壳无需 Windows 启动提权。
#[cfg(target_os = "macos")]
pub fn elevate_at_startup() {}
