//! Windows 管理员权限（提权）相关逻辑。
//!
//! 在 `main` 入口检测到非管理员时用 `ShellExecuteExW(runas)` 自提权重启；
//! 用户取消 UAC 则继续以普通权限运行（可降级，不会因此无法启动）。

use std::os::windows::ffi::OsStrExt;

use anyhow::{Context, Result};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
    UI::Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
    UI::WindowsAndMessaging::SW_SHOWNORMAL,
};
use windows::core::{PCWSTR, w};

/// 当前进程是否以管理员权限运行（查询进程 token 的提升状态）。
pub(in crate::windows_ops) fn is_elevated() -> bool {
    let mut token = HANDLE::default();
    let current_process = unsafe { GetCurrentProcess() };
    if unsafe { OpenProcessToken(current_process, TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut return_length = 0u32;
    unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )
    }
    .is_ok()
        && elevation.TokenIsElevated != 0
}

/// 以管理员权限重新启动当前应用（触发 UAC 弹窗）；调用方随后应退出当前进程。
///
/// 返回 `Ok` 表示新进程已成功启动；用户取消 UAC 时返回 `Err`。
pub(in crate::windows_ops) fn restart_as_admin() -> Result<()> {
    let exe_path = std::env::current_exe().context("获取程序路径失败")?;
    let exe_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    const VERB: PCWSTR = w!("runas");

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: VERB,
        lpFile: PCWSTR::from_raw(exe_wide.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    unsafe { ShellExecuteExW(&mut info) }.context("以管理员身份启动失败")
}
