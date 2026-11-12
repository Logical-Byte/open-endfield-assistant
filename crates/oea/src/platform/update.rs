//! 更新正常路径保持静默，仅在退出后的阻断性失败时显示原生提示。
#[cfg(all(target_os = "windows", not(test)))]
use super::{dialog::DialogIcon, windows};
#[cfg(target_os = "macos")]
use std::process::Command;

pub(crate) mod extra;

pub fn show_update_error(title: &str, content: &str) {
    if cfg!(test) || std::env::var_os("OEA_UPDATE_SILENT").is_some() {
        return;
    }
    #[cfg(all(target_os = "windows", not(test)))]
    let result = windows::update::show_message(title, content, DialogIcon::Error);
    #[cfg(all(target_os = "windows", test))]
    let result: std::io::Result<()> = {
        let _ = (title, content);
        Ok(())
    };
    #[cfg(target_os = "macos")]
    let result = {
        let script = format!(
            "display dialog \"{}\" with title \"{}\" buttons {{\"确定\"}} default button \"确定\"",
            apple_script_string(content),
            apple_script_string(title)
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|_| ())
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result: std::io::Result<()> = {
        let _ = (title, content);
        Ok(())
    };
    if let Err(error) = result {
        tracing::warn!("显示更新失败提示失败: {error}");
    }
}

#[cfg(target_os = "macos")]
pub(super) fn apple_script_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
