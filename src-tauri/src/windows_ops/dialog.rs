//! Windows 原生对话框接口。

use anyhow::Result;

#[cfg(target_os = "windows")]
use super::details;

/// 对话框主图标。
#[derive(Clone, Copy)]
pub enum DialogIcon {
    /// 红色错误图标。
    Error,
    /// 黄色警告图标。
    Warning,
    /// 蓝色信息图标。
    Info,
}

/// 弹出一个仅“确定”按钮的信息对话框。
///
/// `content` 中可用 `<a href="https://...">链接文字</a>` 语法嵌入可点击超链接。
#[cfg(target_os = "windows")]
pub fn show_message(title: &str, content: &str, icon: DialogIcon) -> Result<()> {
    details::dialog::show_message(title, content, icon)
}

/// macOS 开发外壳不提供 Windows 原生对话框。
#[cfg(target_os = "macos")]
pub fn show_message(_title: &str, _content: &str, _icon: DialogIcon) -> Result<()> {
    Err(super::unsupported("native message dialog"))
}

/// 弹出一个“是/否”确认对话框，返回用户是否确认。
#[cfg(target_os = "windows")]
pub(super) fn confirm(title: &str, content: &str, icon: DialogIcon) -> Result<bool> {
    details::dialog::confirm(title, content, icon)
}
