//! 自动更新：安装原语（解压、增量/全量应用、移旧、回滚、清理、配置备份）。
//!
//! 设计要点（v4 §6）：
//! - 内部函数一律显式传路径，便于用临时目录做单元测试；tauri 命令只做薄封装，
//!   用 [`crate::app_paths::AppPaths`] 解析 `<root>/cache` 下的固定目录；
//! - `cache/old` **保留目录结构**：`target/sub/file.txt` 移入后为
//!   `old/sub/file.txt`。这样即使 apply 中途失败（返回 Err 不带部分数据），
//!   前端也能用 `restore_from_old` 把 old 里**全部**内容原样搬回，回滚天然可靠；
//! - `cache/old` 在安装开始前清空一次，安装过程中只写入本次安装的旧文件。

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::app_paths::AppPaths;

/// `changes.json`（Mirror 酱增量包标识，字段名与文档一致：snake_case）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangesJson {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    pub added_dir: Vec<String>,
    pub deleted_dir: Vec<String>,
}

/// 定位应用根目录（dev=项目根 / release=exe 目录）。
fn root_paths() -> Result<AppPaths, String> {
    AppPaths::new().map_err(|e| format!("无法定位应用根目录: {e}"))
}

/// 规范化增量包中的相对路径：循环去掉 `./`、`.\`、`/`、`\` 前缀，防止 join 跳出目标目录。
fn normalize_relative_path(raw: &str) -> &str {
    let mut s = raw.trim();
    loop {
        if let Some(stripped) = s.strip_prefix("./") {
            s = stripped;
        } else if let Some(stripped) = s.strip_prefix(".\\") {
            s = stripped;
        } else if let Some(stripped) = s.strip_prefix('/') {
            s = stripped;
        } else if let Some(stripped) = s.strip_prefix('\\') {
            s = stripped;
        } else {
            break;
        }
    }
    s
}

/// 删除文件或目录（目录递归删除）。
fn remove_path(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// 将 `target_root` 下的文件/目录移动到 `old_dir`（保留相对目录结构），返回实际旧路径。
///
/// 重名时在文件名后追加 `.bak001` 等后缀。`old_dir` 由安装开始前的 `cleanup_old_dir`
/// 统一清空一次，安装过程中移入的内容全部保留，供失败回滚。
pub fn move_to_old_folder(
    source: &Path,
    target_root: &Path,
    old_dir: &Path,
) -> Result<PathBuf, String> {
    if !source.exists() {
        return Err(format!("源文件不存在: {}", source.display()));
    }
    let rel = source.strip_prefix(target_root).map_err(|_| {
        format!(
            "源路径不在目标目录内: {} (target_root={})",
            source.display(),
            target_root.display()
        )
    })?;

    fs::create_dir_all(old_dir)
        .map_err(|e| format!("无法创建 old 目录 [{}]: {e}", old_dir.display()))?;

    let mut dest = old_dir.join(rel);
    if dest.exists() {
        let parent = dest.parent().unwrap_or(old_dir).to_path_buf();
        let file_name = dest
            .file_name()
            .ok_or_else(|| format!("无法获取文件名: {}", dest.display()))?
            .to_string_lossy()
            .into_owned();
        let base = file_name;
        let mut renamed = false;
        for i in 1..=999 {
            let candidate = parent.join(format!("{base}.bak{i:03}"));
            if !candidate.exists() {
                dest = candidate;
                renamed = true;
                break;
            }
        }
        if !renamed {
            return Err(format!("old 目录中同名及 999 个备份均存在: {base}"));
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建 old 子目录 [{}]: {e}", parent.display()))?;
    }

    fs::rename(source, &dest)
        .map_err(|e| format!("无法移动 [{}] -> [{}]: {e}", source.display(), dest.display()))?;
    info!("已移入 old: {} -> {}", source.display(), dest.display());
    Ok(dest)
}

/// 复制单个文件：目标存在时先移入 `old`（失败兜底删除后直接覆盖，保证新文件落盘）。
fn copy_file_with_move_old(
    src: &Path,
    dst: &Path,
    target_root: &Path,
    old_dir: &Path,
) -> Result<(), String> {
    if dst.exists() {
        match move_to_old_folder(dst, target_root, old_dir) {
            Ok(_) => {}
            Err(e) => {
                warn!("移动旧文件失败（将直接覆盖）: {e}");
                if let Err(del_err) = fs::remove_file(dst) {
                    warn!("删除旧文件也失败（尝试直接覆盖）: {del_err}");
                }
            }
        }
    }
    fs::copy(src, dst).map_err(|e| {
        format!(
            "无法复制文件 [{}] -> [{}]: {e}",
            src.display(),
            dst.display()
        )
    })?;
    Ok(())
}

/// 递归复制目录：目标同名项为文件时先移旧再按目录复制。
fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
    target_root: &Path,
    old_dir: &Path,
) -> Result<(), String> {
    if dst.exists() && !dst.is_dir() {
        match move_to_old_folder(dst, target_root, old_dir) {
            Ok(_) => {}
            Err(e) => {
                warn!("移动旧文件失败（将删除后复制目录）: {e}");
                if let Err(del_err) = fs::remove_file(dst) {
                    warn!("删除旧文件也失败: {del_err}");
                }
            }
        }
    }
    fs::create_dir_all(dst)
        .map_err(|e| format!("无法创建目录 [{}]: {e}", dst.display()))?;

    for entry in fs::read_dir(src).map_err(|e| format!("无法读取目录 [{}]: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("无法读取目录条目: {e}"))?;
        let src_item = entry.path();
        let dst_item = dst.join(entry.file_name());
        if src_item.is_dir() {
            copy_dir_recursive(&src_item, &dst_item, target_root, old_dir)?;
        } else {
            copy_file_with_move_old(&src_item, &dst_item, target_root, old_dir)?;
        }
    }
    Ok(())
}

/// 复制目录内容（不含根目录本身），可跳过指定文件名。
fn copy_dir_contents(
    src: &Path,
    dst: &Path,
    target_root: &Path,
    old_dir: &Path,
    skip: &[&str],
) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("无法创建目录 [{}]: {e}", dst.display()))?;

    for entry in fs::read_dir(src).map_err(|e| format!("无法读取目录 [{}]: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("无法读取目录条目: {e}"))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if skip.contains(&name_str.as_ref()) {
            continue;
        }
        let src_item = entry.path();
        let dst_item = dst.join(&name);
        if src_item.is_dir() {
            copy_dir_recursive(&src_item, &dst_item, target_root, old_dir)?;
        } else {
            copy_file_with_move_old(&src_item, &dst_item, target_root, old_dir)?;
        }
    }
    Ok(())
}

/// 解压 zip 到目标目录（目标存在时先整体清理，保证幂等；`enclosed_name` 防目录穿越）。
pub fn extract_zip_file(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("无法打开 ZIP 文件 [{}]: {e}", zip_path.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 ZIP 文件: {e}"))?;

    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)
            .map_err(|e| format!("无法清理解压目录 [{}]: {e}", dest_dir.display()))?;
    }
    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("无法创建解压目录 [{}]: {e}", dest_dir.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("无法读取 ZIP 条目 {i}: {e}"))?;
        let Some(rel_path) = entry.enclosed_name() else {
            warn!("跳过不安全的 ZIP 条目: {}", entry.name());
            continue;
        };
        let out_path = dest_dir.join(rel_path);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("无法创建目录 [{}]: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("无法创建父目录 [{}]: {e}", parent.display()))?;
            }
            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| format!("无法创建文件 [{}]: {e}", out_path.display()))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("无法写入文件 [{}]: {e}", out_path.display()))?;
        }
    }
    info!("解压完成: {} -> {}", zip_path.display(), dest_dir.display());
    Ok(())
}

/// 读取解压目录中的 `changes.json`（不存在返回 `None`）。
pub fn read_changes_json(extract_dir: &Path) -> Result<Option<ChangesJson>, String> {
    let changes_path = extract_dir.join("changes.json");
    if !changes_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&changes_path)
        .map_err(|e| format!("无法读取 changes.json: {e}"))?;
    let changes: ChangesJson =
        serde_json::from_str(&content).map_err(|e| format!("无法解析 changes.json: {e}"))?;
    Ok(Some(changes))
}

/// 应用增量更新：删除 `deleted` 列表（移 old，失败兜底删），再复制解压内容（跳过 `changes.json`）。
pub fn apply_incremental(
    extract_dir: &Path,
    target_dir: &Path,
    old_dir: &Path,
    deleted: &[String],
) -> Result<(), String> {
    info!(
        "应用增量更新: extract={} target={}",
        extract_dir.display(),
        target_dir.display()
    );

    // 1. 删除 changes.json 中列出的废弃文件
    for raw in deleted {
        let normalized = normalize_relative_path(raw);
        let file_path = target_dir.join(normalized);
        if !file_path.exists() {
            continue;
        }
        match move_to_old_folder(&file_path, target_dir, old_dir) {
            Ok(_) => {}
            Err(e) => {
                warn!("移动旧文件失败（将兜底删除）: {e}");
                if let Err(del_err) = remove_path(&file_path) {
                    warn!("兜底删除失败（可能残留旧文件）: {del_err}");
                }
            }
        }
    }

    // 2. 复制解压内容（增量包只含变更文件）
    copy_dir_contents(extract_dir, target_dir, target_dir, old_dir, &["changes.json"])
}

/// 应用全量更新：顶层同名项整体移 old（失败兜底删），再复制解压内容（跳过 `changes.json`）。
pub fn apply_full(
    extract_dir: &Path,
    target_dir: &Path,
    old_dir: &Path,
) -> Result<(), String> {
    info!(
        "应用全量更新: extract={} target={}",
        extract_dir.display(),
        target_dir.display()
    );

    for entry in fs::read_dir(extract_dir)
        .map_err(|e| format!("无法读取解压目录: {e}"))?
    {
        let entry = entry.map_err(|e| format!("无法读取目录条目: {e}"))?;
        let name = entry.file_name();
        if name == "changes.json" {
            continue;
        }
        let target_item = target_dir.join(&name);
        if !target_item.exists() {
            continue;
        }
        match move_to_old_folder(&target_item, target_dir, old_dir) {
            Ok(_) => {}
            Err(e) => {
                warn!("移动旧文件失败（将兜底删除）: {e}");
                if let Err(del_err) = remove_path(&target_item) {
                    warn!("兜底删除失败（可能残留旧文件）: {del_err}");
                }
            }
        }
    }

    copy_dir_contents(extract_dir, target_dir, target_dir, old_dir, &["changes.json"])
}

/// 把 `old_dir` 中的全部内容原样搬回 `target_dir`（尽力而为，逐项容错）。
///
/// 由构造保证路径安全：相对路径来自遍历 `old_dir` 本身，目标始终 join 在 `target_dir` 下。
pub fn restore_from_old_files(target_dir: &Path, old_dir: &Path) -> Result<(), String> {
    if !old_dir.exists() {
        return Ok(());
    }

    fn restore_recursive(
        old_dir: &Path,
        target_dir: &Path,
        rel: &Path,
        failures: &mut usize,
    ) {
        let old_path = old_dir.join(rel);
        let target_path = target_dir.join(rel);

        if old_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&old_path) {
                for entry in entries.flatten() {
                    restore_recursive(old_dir, target_dir, &rel.join(entry.file_name()), failures);
                }
            }
            // 子项已逐个搬回目标目录；目录本身无需改名，搬空后尝试清理（失败无碍，启动清扫兜底）
            let _ = fs::remove_dir(&old_path);
        } else {
            if target_path.exists() {
                if let Err(e) = remove_path(&target_path) {
                    warn!("回滚时清理目标失败: {} ({e})", target_path.display());
                    *failures += 1;
                    return;
                }
            }
            if let Some(parent) = target_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::rename(&old_path, &target_path) {
                warn!("回滚失败: {} ({e})", target_path.display());
                *failures += 1;
            }
        }
    }

    let mut failures = 0usize;
    if let Ok(entries) = fs::read_dir(old_dir) {
        for entry in entries.flatten() {
            restore_recursive(old_dir, target_dir, &PathBuf::from(entry.file_name()), &mut failures);
        }
    }
    if failures > 0 {
        return Err(format!("{failures} 个文件回滚失败"));
    }
    Ok(())
}

/// 备份配置目录到 `backup_dir`（备份前覆盖旧备份）。
pub fn backup_config_dir(config_dir: &Path, backup_dir: &Path) -> Result<(), String> {
    if !config_dir.exists() {
        info!("config 目录不存在，跳过备份");
        return Ok(());
    }
    if backup_dir.exists() {
        fs::remove_dir_all(backup_dir)
            .map_err(|e| format!("无法清理旧配置备份 [{}]: {e}", backup_dir.display()))?;
    }
    copy_dir_contents(config_dir, backup_dir, backup_dir, backup_dir, &[])?;
    info!("配置已备份: {} -> {}", config_dir.display(), backup_dir.display());
    Ok(())
}

/// 清空 `cache/old`（安装开始前调用，保证回滚基线干净）。
pub fn clear_old_dir(old_dir: &Path) -> Result<(), String> {
    if old_dir.exists() {
        fs::remove_dir_all(old_dir)
            .map_err(|e| format!("无法清空 old 目录 [{}]: {e}", old_dir.display()))?;
    }
    Ok(())
}

/// 启动清扫：删除 `cache/old`、`cache/update_extract` 与 `cache/downloads/*.downloading`，
/// 保留已下载完成的 zip（pending 安装仍需使用）。
pub fn clear_stale_update_files(cache_dir: &Path) -> Result<(), String> {
    let old_dir = cache_dir.join("old");
    let extract_dir = cache_dir.join("update_extract");
    let downloads_dir = cache_dir.join("downloads");

    for dir in [&old_dir, &extract_dir] {
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
    }
    if downloads_dir.exists() {
        if let Ok(entries) = fs::read_dir(&downloads_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().ends_with(".downloading"))
                {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
    Ok(())
}

// ============ tauri 命令（薄封装） ============

/// 备份配置到 `cache/config_backup`（失败仅 warn，不阻断安装）。
#[tauri::command]
pub fn backup_config() -> Result<(), String> {
    let paths = root_paths()?;
    match backup_config_dir(&paths.config_dir(), &paths.cache_dir().join("config_backup")) {
        Ok(()) => Ok(()),
        Err(e) => {
            warn!("备份配置失败（不阻断安装）: {e}");
            Ok(())
        }
    }
}

/// 解压更新包到 `cache/update_extract`。
#[tauri::command]
pub fn extract_zip(zip_path: String) -> Result<(), String> {
    let paths = root_paths()?;
    extract_zip_file(Path::new(&zip_path), &paths.cache_dir().join("update_extract"))
}

/// 检查解压目录中是否存在 `changes.json`。
#[tauri::command]
pub fn check_changes_json() -> Result<Option<ChangesJson>, String> {
    let paths = root_paths()?;
    read_changes_json(&paths.cache_dir().join("update_extract"))
}

/// 应用增量更新。
#[tauri::command]
pub fn apply_incremental_update(deleted: Vec<String>) -> Result<(), String> {
    let paths = root_paths()?;
    let cache = paths.cache_dir();
    apply_incremental(
        &cache.join("update_extract"),
        paths.root_dir(),
        &cache.join("old"),
        &deleted,
    )
}

/// 应用全量更新。
#[tauri::command]
pub fn apply_full_update() -> Result<(), String> {
    let paths = root_paths()?;
    let cache = paths.cache_dir();
    apply_full(
        &cache.join("update_extract"),
        paths.root_dir(),
        &cache.join("old"),
    )
}

/// 把 `cache/old` 中的全部内容原样搬回根目录（安装失败时调用）。
#[tauri::command]
pub fn restore_from_old() -> Result<(), String> {
    let paths = root_paths()?;
    restore_from_old_files(paths.root_dir(), &paths.cache_dir().join("old"))
}

/// 清空 `cache/old`（安装开始前调用）。
#[tauri::command]
pub fn cleanup_old_dir() -> Result<(), String> {
    let paths = root_paths()?;
    clear_old_dir(&paths.cache_dir().join("old"))
}

/// 删除解压目录 `cache/update_extract`（安装成功后调用）。
#[tauri::command]
pub fn cleanup_extract_dir() -> Result<(), String> {
    let paths = root_paths()?;
    let extract = paths.cache_dir().join("update_extract");
    if extract.exists() {
        fs::remove_dir_all(&extract)
            .map_err(|e| format!("无法清理解压目录 [{}]: {e}", extract.display()))?;
    }
    Ok(())
}

/// 删除已安装成功的下载包（仅允许删除 `<root>/cache/downloads` 下的文件）。
#[tauri::command]
pub fn remove_downloaded_package(save_path: String) -> Result<(), String> {
    let paths = root_paths()?;
    let downloads = paths.cache_dir().join("downloads");
    let path = Path::new(&save_path);
    if !path.starts_with(&downloads) {
        return Err(format!("拒绝删除下载目录之外的文件: {save_path}"));
    }
    match fs::remove_file(path) {
        Ok(()) => {
            info!("已删除已安装的更新包: {}", path.display());
            Ok(())
        }
        Err(e) => Err(format!("删除更新包失败: {e}")),
    }
}

/// 校验待安装包仍存在（仅允许检查 `<root>/cache/downloads` 下的文件）。
#[tauri::command]
pub fn pending_package_exists(save_path: String) -> bool {
    let Ok(paths) = root_paths() else {
        return false;
    };
    let downloads = paths.cache_dir().join("downloads");
    let path = Path::new(&save_path);
    path.starts_with(&downloads) && path.is_file()
}

/// 启动清扫：`cache/old`、`cache/update_extract` 与 `downloads/*.downloading`。
#[tauri::command]
pub fn cleanup_stale_update_files() -> Result<(), String> {
    let paths = root_paths()?;
    clear_stale_update_files(&paths.cache_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    /// 在系统临时目录创建唯一测试根目录。
    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "oea-update-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn make_zip(zip_path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, content) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn test_normalize_relative_path() {
        assert_eq!(normalize_relative_path("./a/b.txt"), "a/b.txt");
        assert_eq!(normalize_relative_path(".\\a\\b.txt"), "a\\b.txt");
        assert_eq!(normalize_relative_path("/a/b.txt"), "a/b.txt");
        assert_eq!(normalize_relative_path("\\a/b.txt"), "a/b.txt");
        assert_eq!(normalize_relative_path("a/b.txt"), "a/b.txt");
    }

    #[test]
    fn test_extract_zip_with_traversal_guard() {
        let root = temp_root("extract");
        let zip_path = root.join("pkg.zip");
        let dest = root.join("dest");
        make_zip(
            &zip_path,
            &[
                ("a.txt", "aaa"),
                ("sub/b.txt", "bbb"),
                ("../evil.txt", "evil"),
            ],
        );

        extract_zip_file(&zip_path, &dest).unwrap();
        assert!(dest.join("a.txt").exists());
        assert!(dest.join("sub/b.txt").exists());
        // 目录穿越条目被拒绝，不得落在 dest 之外
        assert!(!root.join("evil.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_move_to_old_folder_preserves_structure() {
        let root = temp_root("move-old");
        let target = root.join("target");
        let old = root.join("old");
        write_file(&target.join("sub/file.txt"), "v1");

        let moved = move_to_old_folder(&target.join("sub/file.txt"), &target, &old).unwrap();
        assert_eq!(moved, old.join("sub/file.txt"));
        assert!(!target.join("sub/file.txt").exists());
        assert_eq!(fs::read_to_string(&moved).unwrap(), "v1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_move_to_old_folder_suffix() {
        let root = temp_root("move-old-suffix");
        let target = root.join("target");
        let old = root.join("old");
        write_file(&old.join("sub/file.txt"), "occupied");
        write_file(&target.join("sub/file.txt"), "v2");

        let moved = move_to_old_folder(&target.join("sub/file.txt"), &target, &old).unwrap();
        assert_eq!(moved, old.join("sub/file.txt.bak001"));
        assert_eq!(fs::read_to_string(&moved).unwrap(), "v2");
        assert_eq!(fs::read_to_string(old.join("sub/file.txt")).unwrap(), "occupied");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_apply_incremental_update() {
        let root = temp_root("incremental");
        let target = root.join("target");
        let extract = root.join("extract");
        let old = root.join("old");
        write_file(&target.join("old_a.txt"), "will-be-deleted");
        write_file(&target.join("old_b.txt"), "old-content");
        write_file(&target.join("keep.txt"), "keep");
        write_file(&extract.join("old_b.txt"), "new-content");
        write_file(&extract.join("new_c.txt"), "new-file");
        write_file(&extract.join("changes.json"), "{}");

        let deleted = vec!["old_a.txt".to_string()];
        apply_incremental(&extract, &target, &old, &deleted).unwrap();

        assert_eq!(
            fs::read_to_string(target.join("old_b.txt")).unwrap(),
            "new-content"
        );
        assert!(target.join("new_c.txt").exists());
        assert!(target.join("keep.txt").exists());
        assert!(!target.join("old_a.txt").exists());
        assert!(!target.join("changes.json").exists());
        // 被删除文件与旧版本文件都应进入 old（保留目录结构）
        assert!(old.join("old_a.txt").exists());
        assert!(old.join("old_b.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_apply_full_update() {
        let root = temp_root("full");
        let target = root.join("target");
        let extract = root.join("extract");
        let old = root.join("old");
        write_file(&target.join("models/old-model.onnx"), "old");
        write_file(&target.join("stale.dat"), "stale");
        write_file(&extract.join("models/new-model.onnx"), "new");
        write_file(&extract.join("OEA.exe"), "exe");

        apply_full(&extract, &target, &old).unwrap();

        assert!(target.join("models/new-model.onnx").exists());
        assert!(!target.join("models/old-model.onnx").exists());
        assert!(target.join("OEA.exe").exists());
        // 顶层同名项整体移入 old（结构保留）；`stale.dat` 不在新包中则保留
        assert!(old.join("models/old-model.onnx").exists());
        assert!(target.join("stale.dat").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_restore_from_old() {
        let root = temp_root("restore");
        let target = root.join("target");
        let extract = root.join("extract");
        let old = root.join("old");
        fs::create_dir_all(&extract).unwrap();
        write_file(&target.join("sub/a.txt"), "old");

        apply_incremental(
            &extract,
            &target,
            &old,
            &["sub/a.txt".to_string()],
        )
        .unwrap();
        // 模拟"复制新文件成功后某一步失败"的中间态
        write_file(&target.join("sub/a.txt"), "new");
        write_file(&target.join("sub/new_c.txt"), "partial");

        restore_from_old_files(&target, &old).unwrap();
        assert_eq!(fs::read_to_string(target.join("sub/a.txt")).unwrap(), "old");
        // 中间态残留的新文件不会被回滚（不属于 old），回滚只负责搬回旧文件
        assert!(target.join("sub/new_c.txt").exists());
        assert!(!old.exists() || !old.join("sub/a.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_backup_and_cleanup() {
        let root = temp_root("backup");
        let config = root.join("config");
        let backup = root.join("config_backup");
        let cache = root.join("cache");
        write_file(&config.join("oea_config.json"), r#"{"a":1}"#);
        write_file(&config.join("sub/extra.txt"), "x");

        backup_config_dir(&config, &backup).unwrap();
        assert_eq!(
            fs::read_to_string(backup.join("oea_config.json")).unwrap(),
            r#"{"a":1}"#
        );
        assert!(backup.join("sub/extra.txt").exists());

        // 启动清扫：清 old / update_extract / *.downloading，保留已下载 zip
        write_file(&cache.join("old/old.exe"), "old");
        write_file(&cache.join("update_extract/x.txt"), "x");
        write_file(&cache.join("downloads/pkg.zip.downloading"), "partial");
        write_file(&cache.join("downloads/pkg.zip"), "done");
        clear_stale_update_files(&cache).unwrap();
        assert!(!cache.join("old").exists());
        assert!(!cache.join("update_extract").exists());
        assert!(!cache.join("downloads/pkg.zip.downloading").exists());
        assert!(cache.join("downloads/pkg.zip").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
