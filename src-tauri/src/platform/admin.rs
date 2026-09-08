//! Windows 管理员权限接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::windows;

/// 当前进程是否以管理员权限运行；macOS 开发外壳返回 `false`。
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::admin::is_elevated()
    }

    #[cfg(target_os = "macos")]
    {
        false
    }
}

/// 以管理员权限重新启动当前应用；调用方随后应退出当前进程。
///
/// 返回 `Ok` 表示新进程已成功启动；用户取消 UAC 时返回 `Err`。
/// macOS 开发外壳返回 unsupported error。
pub fn restart_as_admin() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::admin::restart_as_admin()
    }

    #[cfg(target_os = "macos")]
    {
        Err(super::unsupported("restart_as_admin"))
    }
}

/// 启动时自动请求管理员权限（仅 release 生效）。
///
/// 非管理员时自提权重启并退出当前进程；用户取消 UAC 时继续以普通权限运行。
/// debug 构建不处理，方便 `tauri dev` 在普通终端调试。
/// macOS 开发外壳不执行任何操作。
pub fn elevate_at_startup() {
    #[cfg(target_os = "windows")]
    {
        if !cfg!(debug_assertions) && !is_elevated() && restart_as_admin().is_ok() {
            std::process::exit(0);
        }
    }
}
