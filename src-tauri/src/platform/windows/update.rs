//! Windows 更新文件原语。

use std::{
    fs::File,
    io,
    os::windows::{ffi::OsStrExt, io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};

// Cargo 的 Windows 测试 harness 没有 Tauri exe 携带的 Common Controls v6 manifest。
// 测试本来就使用静默提示，因此不要把 `TaskDialogIndirect` 静态链接进测试 exe，
// 否则 Windows loader 会在测试启动前以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出。
#[cfg(not(test))]
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use ::windows::{
    Win32::{
        Foundation::{ERROR_LOCK_VIOLATION, HANDLE},
        Storage::FileSystem::{
            LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, ReplaceFileW,
            UnlockFileEx,
        },
        System::IO::OVERLAPPED,
    },
    core::{PCWSTR, Result as WindowsResult},
};

#[cfg(not(test))]
use ::windows::Win32::{
    Foundation::{LPARAM, S_FALSE, S_OK, WPARAM},
    UI::{
        Controls::{
            TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TDF_CALLBACK_TIMER,
            TDF_SHOW_MARQUEE_PROGRESS_BAR, TDM_CLICK_BUTTON, TDN_BUTTON_CLICKED, TDN_TIMER,
            TaskDialogIndirect,
        },
        WindowsAndMessaging::{IDOK, PostMessageW},
    },
};

#[cfg(not(test))]
use crate::platform::dialog::{self, DialogIcon};

/// 让开发者选择一个 ZIP 更新包，并确认将退出应用执行安装。
pub(in crate::platform) fn choose_update_package(
    default_directory: &Path,
) -> io::Result<Option<PathBuf>> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let default_directory = default_directory.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$picker = New-Object System.Windows.Forms.OpenFileDialog
$picker.Title = '选择用于更新测试的 ZIP'
$picker.InitialDirectory = '{default_directory}'
$picker.Filter = 'ZIP 更新包 (*.zip)|*.zip'
if ($picker.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {{ exit 2 }}
$answer = [System.Windows.Forms.MessageBox]::Show("将安装以下本地更新包并退出 OEA：`n`n$($picker.FileName)", 'OEA 开发者选项', [System.Windows.Forms.MessageBoxButtons]::YesNo, [System.Windows.Forms.MessageBoxIcon]::Warning)
if ($answer -ne [System.Windows.Forms.DialogResult]::Yes) {{ exit 2 }}
[Console]::Out.Write($picker.FileName)"#
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Sta", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if output.status.code() == Some(2) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if path.is_empty() {
        Err(io::Error::other("更新包选择器没有返回文件路径"))
    } else {
        Ok(Some(PathBuf::from(path)))
    }
}

/// 获取文件上的 Windows 排他锁。
pub(in crate::platform) fn lock_file(file: &File, nonblocking: bool) -> io::Result<bool> {
    let mut flags = LOCKFILE_EXCLUSIVE_LOCK;
    if nonblocking {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    let mut overlapped = OVERLAPPED::default();
    let result = unsafe {
        LockFileEx(
            HANDLE(file.as_raw_handle()),
            flags,
            None,
            1,
            0,
            &mut overlapped,
        )
    };

    match result {
        Ok(()) => Ok(true),
        Err(error) if nonblocking && (error.code().0 as u32 & 0xffff) == ERROR_LOCK_VIOLATION.0 => {
            Ok(false)
        }
        Err(error) => Err(io::Error::other(error.to_string())),
    }
}

/// 释放文件上的 Windows 排他锁。
pub(in crate::platform) fn unlock_file(file: &File) -> io::Result<()> {
    let mut overlapped = OVERLAPPED::default();
    unsafe { UnlockFileEx(HANDLE(file.as_raw_handle()), None, 1, 0, &mut overlapped) }
        .map_err(|error| io::Error::other(error.to_string()))
}

/// 用 Windows 的原子替换接口替换应用 exe。
pub(in crate::platform) fn replace_file(replacement: &Path, target: &Path) -> io::Result<()> {
    if !target.exists() {
        return std::fs::rename(replacement, target);
    }

    let replacement_wide = path_to_wide(replacement);
    let target_wide = path_to_wide(target);
    replace_file_w(
        PCWSTR::from_raw(target_wide.as_ptr()),
        PCWSTR::from_raw(replacement_wide.as_ptr()),
        PCWSTR::null(),
    )
    .map_err(|error| io::Error::other(error.to_string()))
}

fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn replace_file_w(target: PCWSTR, replacement: PCWSTR, backup: PCWSTR) -> WindowsResult<()> {
    unsafe { ReplaceFileW(target, replacement, backup, Default::default(), None, None) }
}

/// 一个非阻塞的 Windows 原生 Task Dialog 状态窗。
///
/// `TaskDialogIndirect` 自身是阻塞 API，因此状态窗在独立线程中运行；主流程通过
/// callback timer 关闭它。用户在提交期间触发按钮或关闭请求时，callback 返回
/// `S_FALSE` 保持窗口打开。helper 可以继续替换 exe，测试模式则完全不创建窗口。
#[cfg(not(test))]
pub(in crate::platform) struct StatusWindow {
    close: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

#[cfg(not(test))]
impl StatusWindow {
    pub(in crate::platform) fn show(title: &str, content: &str) -> io::Result<Self> {
        let title = title.to_owned();
        let content = content.to_owned();
        let close = Arc::new(AtomicBool::new(false));
        let close_for_thread = Arc::clone(&close);
        let thread = thread::Builder::new()
            .name("oea-update-status".to_string())
            .spawn(move || {
                let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                let content_wide: Vec<u16> =
                    content.encode_utf16().chain(std::iter::once(0)).collect();
                let callback_data = Box::new(Arc::clone(&close_for_thread));
                let callback_data_ptr = (&*callback_data as *const Arc<AtomicBool>) as isize;
                let config = TASKDIALOGCONFIG {
                    cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
                    dwFlags: TDF_CALLBACK_TIMER | TDF_SHOW_MARQUEE_PROGRESS_BAR,
                    pszWindowTitle: PCWSTR(title_wide.as_ptr()),
                    pszMainInstruction: PCWSTR(title_wide.as_ptr()),
                    pszContent: PCWSTR(content_wide.as_ptr()),
                    pfCallback: Some(status_callback),
                    lpCallbackData: callback_data_ptr,
                    ..Default::default()
                };
                let _ = unsafe { TaskDialogIndirect(&config, None, None, None) };
                drop(callback_data);
            })
            .map_err(|error| io::Error::other(format!("创建更新状态窗线程失败: {error}")))?;
        Ok(Self {
            close,
            thread: Some(thread),
        })
    }
}

#[cfg(not(test))]
impl Drop for StatusWindow {
    fn drop(&mut self) {
        self.close.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(not(test))]
unsafe extern "system" fn status_callback(
    hwnd: ::windows::Win32::Foundation::HWND,
    message: TASKDIALOG_NOTIFICATIONS,
    _wparam: WPARAM,
    _lparam: LPARAM,
    callback_data: isize,
) -> ::windows::core::HRESULT {
    if message == TDN_TIMER {
        let Some(close_pointer) = std::ptr::NonNull::new(callback_data as *mut Arc<AtomicBool>)
        else {
            return S_FALSE;
        };
        let close = unsafe { close_pointer.as_ref() };
        if close.load(Ordering::Acquire) {
            let _ = unsafe {
                PostMessageW(
                    Some(hwnd),
                    TDM_CLICK_BUTTON.0 as u32,
                    WPARAM(IDOK.0 as usize),
                    LPARAM(0),
                )
            };
        }
    }
    if message == TDN_BUTTON_CLICKED {
        let Some(close_pointer) = std::ptr::NonNull::new(callback_data as *mut Arc<AtomicBool>)
        else {
            return S_FALSE;
        };
        let close = unsafe { close_pointer.as_ref() };
        if !close.load(Ordering::Acquire) {
            return S_FALSE;
        }
    }
    S_OK
}

/// 用已有 Windows 原生 Task Dialog 显示最终结果。
#[cfg(not(test))]
pub(in crate::platform) fn show_message(
    title: &str,
    content: &str,
    icon: DialogIcon,
) -> io::Result<()> {
    dialog::show_message(title, content, icon).map_err(|error| io::Error::other(error.to_string()))
}
