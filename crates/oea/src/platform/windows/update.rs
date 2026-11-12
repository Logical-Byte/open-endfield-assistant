//! Windows 更新文件原语。

use std::{
    io,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

/// 选择本地 ZIP。安装确认由共享前端流程负责。
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

/// 更新失败时才显示一次系统原生对话框。
#[cfg(not(test))]
pub(in crate::platform) fn show_message(
    title: &str,
    content: &str,
    icon: crate::platform::dialog::DialogIcon,
) -> io::Result<()> {
    crate::platform::dialog::show_message(title, content, icon)
        .map_err(|error| io::Error::other(error.to_string()))
}
