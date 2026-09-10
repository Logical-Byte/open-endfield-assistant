//! 更新包到完整 candidate 的构造。
//!
//! candidate 构造是一个高层、可能失败的步骤。增量包只从 `baseline` 读取，所有
//! 修改都写入临时 candidate；只有完整 candidate 构造成功后才会把它 rename 到
//! 正式 `candidate/` 并发布 `transaction.json`。因此失败不会留下事务标记，根目录
//! 的 exe 与 resources 也不会被触碰。

use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::info;

use super::workspace::UpdateWorkspace;

/// MirrorChyan 增量包的五个变更列表。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangesJson {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    pub added_dir: Vec<String>,
    pub deleted_dir: Vec<String>,
}

/// 已解压更新包的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    Full,
    Incremental,
}

/// 将 zip 解压到一个新的 package 目录。
///
/// 解压器只接受 zip crate 判定为 enclosed 的路径；`changes.json` 仍保留在 package
/// 中供增量 candidate 构造读取。调用方应在此步骤成功后再调用 [`prepare_candidate`]。
pub fn extract_package_zip(zip_path: &Path, package_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path)
        .map_err(|error| format!("打开更新 ZIP [{}] 失败: {error}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("解析更新 ZIP [{}] 失败: {error}", zip_path.display()))?;
    if package_dir.exists() {
        fs::remove_dir_all(package_dir)
            .map_err(|error| format!("清理旧 package [{}] 失败: {error}", package_dir.display()))?;
    }
    fs::create_dir_all(package_dir).map_err(|error| {
        format!(
            "创建 package 目录 [{}] 失败: {error}",
            package_dir.display()
        )
    })?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("读取更新 ZIP 条目 {index} 失败: {error}"))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(format!("更新 ZIP 含有不安全路径: {}", entry.name()));
        };
        let output = package_dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|error| {
                format!("创建 package 目录失败 [{}]: {error}", output.display())
            })?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!("创建 package 父目录失败 [{}]: {error}", parent.display())
                })?;
            }
            let mut output_file = fs::File::create(&output).map_err(|error| {
                format!("创建 package 文件失败 [{}]: {error}", output.display())
            })?;
            std::io::copy(&mut entry, &mut output_file).map_err(|error| {
                format!("写入 package 文件失败 [{}]: {error}", output.display())
            })?;
            restore_unix_mode(&output_file, entry.unix_mode()).map_err(|error| {
                format!("恢复 package 文件权限失败 [{}]: {error}", output.display())
            })?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn restore_unix_mode(file: &fs::File, mode: Option<u32>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = mode {
        file.set_permissions(fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn restore_unix_mode(_file: &fs::File, _mode: Option<u32>) -> std::io::Result<()> {
    Ok(())
}

impl PackageKind {
    pub fn detect(package_dir: &Path) -> Self {
        if package_dir.join("changes.json").is_file() {
            Self::Incremental
        } else {
            Self::Full
        }
    }
}

/// 从已解压的 package 构造并发布一个完整 candidate。
///
/// 增量包会先复制当前根目录下的 exe/resources 到 `baseline/`，然后执行一次只读
/// baseline 的高层构造步骤。全量包直接构造 candidate，不创建 baseline。任一步失
/// 败都会删除临时 candidate 和 baseline，不会创建 `transaction.json`。
pub fn prepare_candidate(
    workspace: &UpdateWorkspace,
    package_dir: &Path,
) -> Result<PackageKind, String> {
    workspace
        .ensure_update_path()
        .map_err(|error| format!("创建更新工作区失败: {error}"))?;
    if workspace.transaction_exists() {
        return Err("已有未完成的更新事务，不能准备新的 candidate".to_string());
    }

    // 上一次准备或 helper 失败已经清理 transaction；这些目录不再有消费者，可以安全重建。
    remove_directory_if_present(&workspace.baseline_path())?;
    remove_directory_if_present(&workspace.candidate_path())?;
    remove_directory_if_present(&workspace.discard_path())?;
    remove_directory_if_present(&workspace.candidate_build_path())?;

    let kind = PackageKind::detect(package_dir);
    let result = prepare_candidate_inner(workspace, package_dir, kind);
    if result.is_err() {
        let _ = remove_directory_if_present(&workspace.baseline_path());
        let _ = remove_directory_if_present(&workspace.candidate_path());
        let _ = remove_directory_if_present(&workspace.candidate_build_path());
        let _ = remove_directory_if_present(&workspace.discard_path());
        let _ = workspace.remove_transaction();
    }
    result
}

fn prepare_candidate_inner(
    workspace: &UpdateWorkspace,
    package_dir: &Path,
    kind: PackageKind,
) -> Result<PackageKind, String> {
    let building = workspace.candidate_build_path();
    fs::create_dir_all(&building)
        .map_err(|error| format!("创建 candidate 临时目录失败: {error}"))?;

    if !package_dir.is_dir() {
        return Err(format!("更新 package 不是目录: {}", package_dir.display()));
    }

    let result = match kind {
        PackageKind::Full => construct_full_candidate(workspace, package_dir, &building),
        PackageKind::Incremental => {
            copy_current_payload_to_baseline(workspace)?;
            construct_incremental_candidate(workspace, package_dir, &building)
        }
    };
    result?;

    validate_candidate(workspace, &building)?;
    fs::rename(&building, workspace.candidate_path()).map_err(|error| {
        format!(
            "发布 candidate 目录失败 [{}] -> [{}]: {error}",
            building.display(),
            workspace.candidate_path().display()
        )
    })?;

    if let Err(error) = workspace.publish_transaction() {
        let _ = remove_directory_if_present(&workspace.candidate_path());
        let _ = remove_directory_if_present(&workspace.baseline_path());
        return Err(error);
    }

    info!(
        "更新 candidate 已发布: {} ({kind:?})",
        workspace.candidate_path().display()
    );
    Ok(kind)
}

/// 复制当前 exe/resources 到增量包的 baseline。baseline 只供 candidate 构造读取。
fn copy_current_payload_to_baseline(workspace: &UpdateWorkspace) -> Result<(), String> {
    let baseline = workspace.baseline_path();
    fs::create_dir_all(&baseline).map_err(|error| format!("创建 baseline 目录失败: {error}"))?;
    copy_regular_file(
        &workspace.executable_path(),
        &baseline.join(workspace.executable_name()),
    )?;
    copy_directory(&workspace.resources_path(), &baseline.join("resources"))?;
    Ok(())
}

/// 全量包只读取 package 的 exe/resources，构造出 candidate 所需的完整 payload。
fn construct_full_candidate(
    workspace: &UpdateWorkspace,
    package_dir: &Path,
    building: &Path,
) -> Result<(), String> {
    let package_exe = find_package_executable(package_dir, workspace.executable_name())?;
    copy_regular_file(&package_exe, &building.join(workspace.executable_name()))?;
    copy_directory(&package_dir.join("resources"), &building.join("resources"))?;
    Ok(())
}

/// 从 baseline 构造完整 candidate。这个函数只把 baseline 当作读源，所有写入均在 building。
fn construct_incremental_candidate(
    workspace: &UpdateWorkspace,
    package_dir: &Path,
    building: &Path,
) -> Result<(), String> {
    let changes = read_changes_json(package_dir)?;
    let baseline = workspace.baseline_path();
    copy_payload_directory(&baseline, building)?;

    // 文件先于目录删除，否则 `deleted_dir` 删除父目录后，同一目录中的 `deleted`
    // 文件会被误判为缺失。目录按路径深度降序处理，允许清单同时列出父子目录。
    for raw in changes.deleted {
        let relative = validate_payload_path(workspace, &raw, true)?;
        let target = building.join(&relative);
        if !target.is_file() {
            return Err(format!("增量包要删除的文件不存在: {raw}"));
        }
        fs::remove_file(&target)
            .map_err(|error| format!("删除 candidate 文件 [{}] 失败: {error}", target.display()))?;
    }

    let mut deleted_directories = changes
        .deleted_dir
        .into_iter()
        .map(|raw| {
            let relative = validate_payload_path(workspace, &raw, false)?;
            Ok((raw, relative))
        })
        .collect::<Result<Vec<_>, String>>()?;
    deleted_directories
        .sort_by_key(|(_, relative)| std::cmp::Reverse(relative.components().count()));
    for (raw, relative) in deleted_directories {
        let target = building.join(&relative);
        if !target.is_dir() {
            return Err(format!("增量包要删除的目录不存在: {raw}"));
        }
        fs::remove_dir_all(&target)
            .map_err(|error| format!("删除 candidate 目录 [{}] 失败: {error}", target.display()))?;
    }

    for raw in changes.added_dir {
        let relative = validate_payload_path(workspace, &raw, false)?;
        let source = package_dir.join(&relative);
        if source.exists() && !source.is_dir() {
            return Err(format!("增量包 added_dir 不是目录: {raw}"));
        }
        fs::create_dir_all(building.join(&relative))
            .map_err(|error| format!("创建 candidate 目录 [{}] 失败: {error}", raw))?;
    }

    for raw in changes.added.into_iter().chain(changes.modified) {
        let relative = validate_payload_path(workspace, &raw, true)?;
        let source = package_dir.join(&relative);
        if !source.is_file() {
            return Err(format!("增量包文件不存在: {raw}"));
        }
        copy_regular_file(&source, &building.join(&relative))?;
    }

    Ok(())
}

fn read_changes_json(package_dir: &Path) -> Result<ChangesJson, String> {
    let path = package_dir.join("changes.json");
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("读取 changes.json [{}] 失败: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("解析 changes.json 失败: {error}"))
}

/// 严格限制增量字段只能触碰目标 exe 或 resources 子树。
fn validate_payload_path(
    workspace: &UpdateWorkspace,
    raw: &str,
    file_path: bool,
) -> Result<PathBuf, String> {
    let normalized = raw.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.as_bytes().get(1) == Some(&b':')
        || normalized.contains(':')
    {
        return Err(format!("增量包路径不是安全的相对路径: {raw:?}"));
    }

    let mut relative = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            _ => return Err(format!("增量包路径包含非法片段: {raw:?}")),
        }
    }

    let is_executable = relative.as_path() == Path::new(workspace.executable_name());
    let is_resource = relative
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "resources");
    if !is_executable && !is_resource {
        return Err(format!(
            "增量包路径只能指向 {} 或 resources/**: {raw:?}",
            workspace.executable_name().to_string_lossy()
        ));
    }
    if file_path && relative.file_name().is_none() {
        return Err(format!("增量包文件路径不能为空: {raw:?}"));
    }
    if !file_path && is_executable {
        return Err(format!("exe 不是目录，不能出现在目录变更列表: {raw:?}"));
    }
    Ok(relative)
}

fn find_package_executable(package_dir: &Path, expected: &OsString) -> Result<PathBuf, String> {
    let expected_path = package_dir.join(expected);
    if expected_path.is_file() {
        return Ok(expected_path);
    }

    // macOS 构建包可能使用 OEA 或其它当前 binary 名称；允许发布包中的标准 OEA.exe
    // 作为源，但 candidate 仍然使用当前运行 binary 的名称。
    for fallback in [OsString::from("OEA.exe"), OsString::from("OEA")] {
        let path = package_dir.join(&fallback);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "更新 package 缺少可执行文件: {}",
        expected.to_string_lossy()
    ))
}

fn validate_candidate(workspace: &UpdateWorkspace, candidate: &Path) -> Result<(), String> {
    let executable = candidate.join(workspace.executable_name());
    let resources = candidate.join("resources");
    if !executable.is_file() {
        return Err(format!(
            "candidate 缺少可执行文件: {}",
            executable.display()
        ));
    }
    if !resources.is_dir() {
        return Err(format!(
            "candidate 缺少 resources 目录: {}",
            resources.display()
        ));
    }

    for entry in fs::read_dir(candidate).map_err(|error| format!("读取 candidate 失败: {error}"))?
    {
        let entry = entry.map_err(|error| format!("读取 candidate 条目失败: {error}"))?;
        let name = entry.file_name();
        if name.as_os_str() != workspace.executable_name().as_os_str() && name != "resources" {
            return Err(format!(
                "candidate 含有未允许的顶层条目: {}",
                name.to_string_lossy()
            ));
        }
    }
    Ok(())
}

fn copy_payload_directory(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("创建 candidate 目录 [{}] 失败: {error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("读取 baseline 目录 [{}] 失败: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("读取 baseline 条目失败: {error}"))?;
        let source_item = entry.path();
        let target_item = target.join(entry.file_name());
        if source_item.is_dir() {
            copy_directory(&source_item, &target_item)?;
        } else {
            copy_regular_file(&source_item, &target_item)?;
        }
    }
    Ok(())
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!("目录不存在: {}", source.display()));
    }
    if fs::symlink_metadata(source)
        .map_err(|error| format!("读取目录元数据失败: {error}"))?
        .file_type()
        .is_symlink()
    {
        return Err(format!("拒绝复制符号链接目录: {}", source.display()));
    }
    fs::create_dir_all(target)
        .map_err(|error| format!("创建目录 [{}] 失败: {error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("读取目录 [{}] 失败: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("读取目录条目失败: {error}"))?;
        let source_item = entry.path();
        let target_item = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_item)
            .map_err(|error| format!("读取源条目元数据失败: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("拒绝复制符号链接: {}", source_item.display()));
        }
        if metadata.is_dir() {
            copy_directory(&source_item, &target_item)?;
        } else if metadata.is_file() {
            copy_regular_file(&source_item, &target_item)?;
        } else {
            return Err(format!("不支持的源条目类型: {}", source_item.display()));
        }
    }
    Ok(())
}

fn copy_regular_file(source: &Path, target: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("读取源文件 [{}] 失败: {error}", source.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("源不是普通文件: {}", source.display()));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建目标父目录 [{}] 失败: {error}", parent.display()))?;
    }
    fs::copy(source, target).map_err(|error| {
        format!(
            "复制文件 [{}] -> [{}] 失败: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn remove_directory_if_present(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("清理目录 [{}] 失败: {error}", path.display()))?;
    }
    Ok(())
}
