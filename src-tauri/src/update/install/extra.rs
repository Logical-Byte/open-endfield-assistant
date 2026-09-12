//! 开发者选项的本地更新包入口。

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::app_paths::AppPaths;

/// 选择本地 ZIP 并复制到受控下载目录，供开发者走完整的生产安装入口。
#[tauri::command]
pub fn developer_choose_update_package() -> Result<Option<String>, String> {
    if cfg!(debug_assertions) {
        return Err("开发构建禁止执行真实自更新，请使用 release 构建验证".to_string());
    }

    tracing::info!("[developer update] Rust picker command reached");
    let paths = AppPaths::new().map_err(|error| format!("无法定位应用目录: {error}"))?;
    let downloads = paths.cache_dir().join("downloads");
    fs::create_dir_all(&downloads).map_err(|error| format!("创建更新下载目录失败: {error}"))?;
    let Some(selected) = crate::platform::update::extra::choose_update_package(&downloads)
        .map_err(|error| format!("选择更新包失败: {error}"))?
    else {
        tracing::info!("[developer update] native picker or confirmation cancelled");
        return Ok(None);
    };
    let staged = stage_developer_package(&paths, &selected)?;
    let staged = staged
        .to_str()
        .ok_or_else(|| "已复制更新包路径不是有效 UTF-8".to_string())?
        .to_owned();
    tracing::info!("[developer update] native confirmation accepted: {staged}");
    Ok(Some(staged))
}

/// 导入用户明确选择的外部包到 `cache/downloads`；目录内的包直接复用。
fn stage_developer_package(paths: &AppPaths, selected: &Path) -> Result<PathBuf, String> {
    if !selected
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err("请选择 .zip 更新包".to_string());
    }

    let downloads = paths.cache_dir().join("downloads");
    fs::create_dir_all(&downloads).map_err(|error| format!("创建更新下载目录失败: {error}"))?;
    let selected = selected
        .canonicalize()
        .map_err(|error| format!("解析所选更新包路径失败: {error}"))?;
    let canonical_downloads = downloads
        .canonicalize()
        .map_err(|error| format!("解析更新下载目录失败: {error}"))?;
    if selected.starts_with(&canonical_downloads) {
        Ok(selected)
    } else {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("生成更新包暂存名称失败: {error}"))?
            .as_nanos();
        let staged = downloads.join(format!(
            "developer-update-{}-{timestamp}.zip",
            std::process::id()
        ));
        copy_to_new_file(&selected, &staged)?;
        staged
            .canonicalize()
            .map_err(|error| format!("解析已复制更新包路径失败: {error}"))
    }
}

/// 以 `create_new` 创建暂存文件，任何命名碰撞都不得覆盖另一个安装正在读取的包。
fn copy_to_new_file(source: &Path, target: &Path) -> Result<(), String> {
    let mut source = File::open(source).map_err(|error| format!("打开所选更新包失败: {error}"))?;
    let mut target_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| format!("创建更新包暂存文件失败: {error}"))?;
    if let Err(error) = io::copy(&mut source, &mut target_file) {
        let _ = fs::remove_file(target);
        return Err(format!("复制所选更新包到下载目录失败: {error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn developer_update_command_rejects_debug_build() {
        let error = developer_choose_update_package().unwrap_err();

        assert_eq!(
            error,
            "开发构建禁止执行真实自更新，请使用 release 构建验证".to_string()
        );
    }

    #[test]
    fn developer_package_is_staged_in_controlled_download_directory() {
        let root = tempfile::Builder::new()
            .prefix("oea-update-developer-package-")
            .tempdir()
            .unwrap();
        let external = tempfile::Builder::new()
            .prefix("oea-update-external-package-")
            .tempdir()
            .unwrap();
        let paths = AppPaths::with_root_dir(root.path());
        let selected = external.path().join("selected.zip");
        fs::write(&selected, "first package").unwrap();

        let first_staged = stage_developer_package(&paths, &selected).unwrap();
        fs::write(&selected, "second package").unwrap();
        let second_staged = stage_developer_package(&paths, &selected).unwrap();

        let downloads = paths.cache_dir().join("downloads").canonicalize().unwrap();
        assert!(first_staged.starts_with(&downloads));
        assert!(second_staged.starts_with(downloads));
        assert_ne!(first_staged, second_staged);
        assert_eq!(fs::read_to_string(first_staged).unwrap(), "first package");
        assert_eq!(fs::read_to_string(second_staged).unwrap(), "second package");
    }
}
