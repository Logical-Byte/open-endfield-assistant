//! Windows 更新文件原语。

use std::{
    fs::File,
    io,
    os::windows::ffi::OsStrExt,
    os::windows::io::AsRawHandle,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use ::windows::{
    Win32::{
        Foundation::{ERROR_LOCK_VIOLATION, HANDLE, LPARAM, S_FALSE, S_OK, WPARAM},
        Storage::FileSystem::{
            LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, ReplaceFileW,
            UnlockFileEx,
        },
        System::IO::OVERLAPPED,
        UI::{
            Controls::{
                TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TDF_CALLBACK_TIMER,
                TDF_SHOW_MARQUEE_PROGRESS_BAR, TDM_CLICK_BUTTON, TDN_BUTTON_CLICKED, TDN_TIMER,
                TaskDialogIndirect,
            },
            WindowsAndMessaging::{IDOK, PostMessageW},
        },
    },
    core::{PCWSTR, Result as WindowsResult},
};

use crate::platform::dialog::{self, DialogIcon};

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
pub(in crate::platform) struct StatusWindow {
    close: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

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

impl Drop for StatusWindow {
    fn drop(&mut self) {
        self.close.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

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
pub(in crate::platform) fn show_message(
    title: &str,
    content: &str,
    icon: DialogIcon,
) -> io::Result<()> {
    dialog::show_message(title, content, icon).map_err(|error| io::Error::other(error.to_string()))
}
