//! Windows 原生 TaskDialog 封装：支持可点击超链接、图标与「是/否」按钮。
//!
//! 替代 `MessageBoxW`：`TDF_ENABLE_HYPERLINKS` 让内容中的
//! `<a href="https://...">链接文字</a>` 渲染为可点击链接，点击后由
//! `ShellExecuteW` 交给系统浏览器打开——与 Tauri 自身在 WebView2 缺失时
//! 弹出的对话框同款实现（参考 tauri-runtime-wry 的 `dialog/windows.rs`）。
//!
//! 不依赖 WebView，可在 Tauri 启动前使用（WebView2 缺失兜底、setup 失败兜底）。

use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, S_OK, WPARAM};
use windows::Win32::UI::Controls::{
    TASKDIALOG_COMMON_BUTTON_FLAGS, TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0,
    TD_ERROR_ICON, TD_INFORMATION_ICON, TD_WARNING_ICON, TDCBF_NO_BUTTON, TDCBF_OK_BUTTON,
    TDCBF_YES_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_ENABLE_HYPERLINKS, TDN_HYPERLINK_CLICKED,
    TaskDialogIndirect,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{IDYES, SW_SHOWNORMAL};
use windows::core::{HRESULT, PCWSTR};

/// 对话框主图标（对应 TaskDialog 的主图标）。
#[derive(Clone, Copy)]
pub enum DialogIcon {
    /// 红色错误图标。
    Error,
    /// 黄色警告图标。
    Warning,
    /// 蓝色信息图标。
    Info,
}

impl DialogIcon {
    fn to_pcwstr(self) -> PCWSTR {
        match self {
            DialogIcon::Error => TD_ERROR_ICON,
            DialogIcon::Warning => TD_WARNING_ICON,
            DialogIcon::Info => TD_INFORMATION_ICON,
        }
    }
}

/// 弹出一个仅「确定」按钮的信息对话框。
///
/// `content` 中可用 `<a href="https://...">链接文字</a>` 语法嵌入可点击超链接。
pub fn show_message(title: &str, content: &str, icon: DialogIcon) -> Result<()> {
    task_dialog(title, content, icon, TDCBF_OK_BUTTON, None, None)?;
    Ok(())
}

/// 弹出一个「是/否」确认对话框（默认焦点在「是」），返回用户是否确认。
///
/// 内容同样支持 `<a href="...">` 超链接；按 Esc 或关闭窗口视为「否」。
pub fn confirm(title: &str, content: &str, icon: DialogIcon) -> Result<bool> {
    let mut selected = 0;
    task_dialog(
        title,
        content,
        icon,
        TDCBF_YES_BUTTON | TDCBF_NO_BUTTON,
        Some(&mut selected),
        Some(IDYES.0),
    )?;
    Ok(selected == IDYES.0)
}

/// `TaskDialogIndirect` 的统一入口。
///
/// `selected` 用于取回用户点击的按钮 ID；`default_button` 指定初始聚焦按钮。
fn task_dialog(
    title: &str,
    content: &str,
    icon: DialogIcon,
    buttons: TASKDIALOG_COMMON_BUTTON_FLAGS,
    selected: Option<&mut i32>,
    default_button: Option<i32>,
) -> windows::core::Result<()> {
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let content_wide: Vec<u16> = content.encode_utf16().chain(std::iter::once(0)).collect();

    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_ENABLE_HYPERLINKS,
        pszWindowTitle: PCWSTR(title_wide.as_ptr()),
        pszContent: PCWSTR(content_wide.as_ptr()),
        Anonymous1: TASKDIALOGCONFIG_0 {
            pszMainIcon: icon.to_pcwstr(),
        },
        dwCommonButtons: buttons,
        nDefaultButton: default_button.unwrap_or(0),
        pfCallback: Some(task_dialog_callback),
        ..Default::default()
    };

    let mut fallback_selected = 0i32;
    let selected_ptr = selected.unwrap_or(&mut fallback_selected);
    unsafe { TaskDialogIndirect(&config, Some(selected_ptr), None, None) }
}

/// 处理 `TDN_HYPERLINK_CLICKED` 通知：用系统浏览器打开链接。
extern "system" fn task_dialog_callback(
    _hwnd: HWND,
    msg: TASKDIALOG_NOTIFICATIONS,
    _wparam: WPARAM,
    lparam: LPARAM,
    _data: isize,
) -> HRESULT {
    if msg == TDN_HYPERLINK_CLICKED {
        // lparam 指向链接的宽字符串（LPCWSTR）
        let link = PCWSTR(lparam.0 as *const u16);
        let _ = unsafe { ShellExecuteW(None, None, link, None, None, SW_SHOWNORMAL) };
    }
    S_OK
}
