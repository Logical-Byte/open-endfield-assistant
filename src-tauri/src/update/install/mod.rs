//! 自动更新的 candidate/transaction 安装核心。
//!
//! 更新事务只有一个发布标记：`cache/update/transaction.json`。candidate 在标记创建
//! 前完整构造，helper 只提交 exe，v2 启动阶段提交 resources；candidate 内尚未移动
//! 的条目代表事务进度。这里不记录版本号、不做启动哈希扫描，也不提供旧版本回滚。

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use serde::Serialize;
use tauri::Emitter;
use tracing::{debug, error, info, warn};

use crate::app_paths::AppPaths;

mod candidate;
pub(crate) mod extra;
mod helper;
mod startup;
mod transaction;
mod workspace;

use candidate::{PackageKind, extract_package_zip, prepare};
use helper::spawn_helper;
pub use helper::{helper_request_from_args, run_helper_request, run_helper_request_with_logging};
pub use startup::StartupUpdateResult;
pub(crate) use startup::complete_startup_transaction;
pub(crate) use workspace::InstallTarget;
use workspace::InstallWorkspace;

static STARTUP_UPDATE_RESULT: OnceLock<Mutex<Option<StartupUpdateResult>>> = OnceLock::new();

fn startup_result_slot() -> &'static Mutex<Option<StartupUpdateResult>> {
    STARTUP_UPDATE_RESULT.get_or_init(|| Mutex::new(None))
}

/// 记录应用启动早期完成的资源事务结果，供 Tauri 前端在初始化后消费。
pub fn record_startup_update_result(result: StartupUpdateResult) {
    if result == StartupUpdateResult::Completed {
        *startup_result_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(result);
    }
}

/// 取出并清除本进程启动早期的更新结果。
#[tauri::command]
pub fn consume_startup_update_result() -> Option<StartupUpdateResult> {
    startup_result_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum InstallStage {
    Preparing,
    Extracting,
    ApplyingIncremental,
    ApplyingFull,
    CleaningUp,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct InstallStageEvent {
    stage: InstallStage,
}

fn emit_install_stage(app: &tauri::AppHandle, stage: InstallStage) {
    debug!(
        operation = "install",
        stage = ?stage,
        "更新安装进入新阶段"
    );
    if let Err(emit_error) = app.emit("update-install-stage", InstallStageEvent { stage }) {
        warn!(
            operation = "install",
            stage = ?stage,
            error = %emit_error,
            "向前端发送更新安装阶段失败，后端继续执行安装"
        );
    }
}

fn validate_download_package(paths: &AppPaths, package_path: &Path) -> Result<PathBuf, String> {
    let downloads = paths.downloads_dir();
    let canonical_downloads = downloads
        .canonicalize()
        .map_err(|error| format!("无法定位更新下载目录: {error}"))?;
    let canonical_path = package_path
        .canonicalize()
        .map_err(|error| format!("更新包不存在: {error}"))?;
    if !canonical_path.starts_with(&canonical_downloads) || !canonical_path.is_file() {
        return Err(format!(
            "更新包不在 cache/downloads 内: {}",
            package_path.display()
        ));
    }
    Ok(canonical_path)
}

/// 构造 candidate、原子发布 transaction、复制并启动 helper，然后请求当前 v1 退出。
///
/// 这是普通自动更新唯一需要调用的安装接口。debug 构建禁止触碰项目根目录；集成测试应直接
/// 使用 candidate、transaction 与 helper 的语义接口的临时
/// workspace 模块接口。
#[tauri::command]
pub fn install_update(
    manager: tauri::State<'_, super::UpdateManager>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let install_lease = manager.start_install().map_err(|install_error| {
        warn!(
            operation = "install",
            error = %install_error,
            "更新安装请求被状态机拒绝"
        );
        install_error.to_string()
    })?;
    debug!(
        operation = "install",
        package = %install_lease.package_path().display(),
        "更新安装开始"
    );
    info!("开始安装更新");
    if let Err(install_error) = install_update_inner(app, install_lease.package_path()) {
        error!(
            operation = "install",
            error = %install_error,
            "更新安装失败"
        );
        return Err(install_error);
    }
    install_lease.complete();
    Ok(())
}

/// 准备更新并把已发布的事务交给 helper。
///
/// 成功路径会在 helper 启动后调用 `app.exit(0)`，请求当前 v1 退出。末尾的 `Ok(())`
/// 只用于满足 Tauri command 的返回类型；前端不应在它之后继续安装流程。
fn install_update_inner(app: tauri::AppHandle, package_path: &Path) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("开发构建禁止执行真实自更新，请使用临时目录集成测试".to_string());
    }

    emit_install_stage(&app, InstallStage::Preparing);
    let paths = AppPaths::new()?;
    let package_zip = validate_download_package(&paths, package_path)?;
    let target = InstallTarget::for_current_executable(&paths)
        .map_err(|error| format!("无法确定应用 executable name: {error}"))?;
    let workspace = InstallWorkspace::new(&target);
    debug!(
        operation = "install",
        package = %package_zip.display(),
        workspace = %paths.cache_dir().join("update").display(),
        "更新安装路径校验完成"
    );
    // 没有正式 transaction 时，这些只能是未发布的准备残留，可以安全抛弃。新的
    // 安装绝不复用已经消费过的 ZIP 或半途 candidate。
    if let Err(error) = transaction::ensure_inactive(&target) {
        let message = format!("开始安装前检查更新事务失败: {error}");
        return match fs::remove_file(&package_zip) {
            Ok(()) => Err(message),
            Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => {
                Err(message)
            }
            Err(remove_error) => Err(format!(
                "{message}；删除下载文件 [{}] 也失败: {remove_error}",
                package_zip.display()
            )),
        };
    }
    if let Err(error) = workspace.cleanup_inactive_material() {
        let message = format!("开始安装前清理旧更新文件失败: {error}");
        return match fs::remove_file(&package_zip) {
            Ok(()) => Err(message),
            Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => {
                Err(message)
            }
            Err(remove_error) => Err(format!(
                "{message}；删除下载文件 [{}] 也失败: {remove_error}",
                package_zip.display()
            )),
        };
    }
    let package_dir = workspace.package_path();

    emit_install_stage(&app, InstallStage::Extracting);
    if let Err(error) = extract_package_zip(&package_zip, &package_dir) {
        return Err(cleanup_failed_preparation(&target, &package_zip, error));
    }

    let kind = PackageKind::detect(&package_dir);
    emit_install_stage(
        &app,
        match kind {
            PackageKind::Full => InstallStage::ApplyingFull,
            PackageKind::Incremental => InstallStage::ApplyingIncremental,
        },
    );
    let (prepared, _) = match prepare(&target, &package_dir) {
        Ok(prepared) => prepared,
        Err(error) => return Err(cleanup_failed_preparation(&target, &package_zip, error)),
    };
    let begun = transaction::begin(prepared)
        .map_err(|error| cleanup_failed_preparation(&target, &package_zip, error))?;

    // transaction 已证明 candidate 完整；zip 从此不能被复用，删除失败则不启动 helper。
    if let Err(error) = fs::remove_file(&package_zip) {
        let message = format!("删除已消费的更新包失败: {error}");
        return match begun.cancel() {
            Ok(()) => Err(message),
            Err(cleanup_error) => Err(format!("{message}；清理事务也失败: {cleanup_error}")),
        };
    }
    debug!(
        operation = "install",
        package = %package_zip.display(),
        "已删除完成消费的更新包"
    );
    emit_install_stage(&app, InstallStage::CleaningUp);
    if let Err(cleanup_error) = fs::remove_dir_all(&package_dir) {
        if cleanup_error.kind() != std::io::ErrorKind::NotFound {
            warn!(
                operation = "install",
                path = %package_dir.display(),
                error = %cleanup_error,
                "清理已解压更新包失败，后端继续交接更新事务"
            );
        }
    }

    let helper = match spawn_helper(&target) {
        Ok(helper) => helper,
        Err(error) => {
            return match begun.cancel() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}；清理事务也失败: {cleanup}")),
            };
        }
    };
    debug!(
        operation = "install",
        helper_pid = helper.id(),
        "更新 helper 已启动，当前进程准备退出"
    );
    info!("更新文件准备完成，正在退出 OEA 并启动安装程序");

    // helper 已经独立接管事务；当前进程必须退出，释放 Windows 对根 exe 的占用。
    app.exit(0);
    Ok(())
}

fn cleanup_failed_preparation(
    target: &InstallTarget,
    package_zip: &Path,
    primary_error: String,
) -> String {
    let mut cleanup_errors = Vec::new();
    if let Err(error) = fs::remove_file(package_zip) {
        if error.kind() != std::io::ErrorKind::NotFound {
            cleanup_errors.push(format!(
                "删除下载文件 [{}] 失败: {error}",
                package_zip.display()
            ));
        }
    }
    if let Err(error) = InstallWorkspace::new(target).cleanup_inactive_material() {
        cleanup_errors.push(error);
    }

    if cleanup_errors.is_empty() {
        primary_error
    } else {
        format!(
            "{primary_error}；清理临时文件也失败: {}",
            cleanup_errors.join("；")
        )
    }
}
