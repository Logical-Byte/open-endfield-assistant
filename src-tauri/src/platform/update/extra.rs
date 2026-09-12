//! 开发者选项使用的原生更新包选择器。

use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(target_os = "macos")]
use std::process::Command;

/// 让开发者选择一个 ZIP 更新包，并在原生窗口中确认将退出应用执行安装。
pub fn choose_update_package(default_directory: &Path) -> io::Result<Option<PathBuf>> {
    #[cfg(target_os = "macos")]
    {
        choose_update_package_macos(default_directory)
    }

    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::update::choose_update_package(default_directory)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = default_directory;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "update package picker is unsupported on this platform",
        ))
    }
}

#[cfg(target_os = "macos")]
fn choose_update_package_macos(default_directory: &Path) -> io::Result<Option<PathBuf>> {
    let default_directory = super::apple_script_string(&default_directory.to_string_lossy());
    let script = format!(
        "set selectedFile to choose file with prompt \"选择用于更新测试的 ZIP\" default location POSIX file \"{default_directory}/\" of type {{\"public.zip-archive\"}}\n\
         set selectedPath to POSIX path of selectedFile\n\
         display dialog \"将安装以下本地更新包并退出 OEA：\\n\\n\" & selectedPath buttons {{\"取消\", \"安装并退出\"}} default button \"安装并退出\" cancel button \"取消\" with title \"OEA 开发者选项\"\n\
         return selectedPath"
    );
    let output = Command::new("osascript").args(["-e", &script]).output()?;
    if !output.status.success() {
        return Ok(None);
    }
    output_path(&output.stdout).map(Some)
}

#[cfg(target_os = "macos")]
fn output_path(stdout: &[u8]) -> io::Result<PathBuf> {
    let path = String::from_utf8_lossy(stdout).trim().to_owned();
    if path.is_empty() {
        Err(io::Error::other("更新包选择器没有返回文件路径"))
    } else {
        Ok(PathBuf::from(path))
    }
}
