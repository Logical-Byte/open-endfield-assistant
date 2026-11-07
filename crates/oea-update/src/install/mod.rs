//! 自动更新的 candidate/transaction 安装核心。
//!
//! 更新事务只有一个发布标记：`cache/update/transaction.json`。candidate 在标记创建
//! 前完整构造，helper 只提交 exe，v2 启动阶段提交 resources；candidate 内尚未移动
//! 的条目代表事务进度。这里不记录版本号、不做启动哈希扫描，也不提供旧版本回滚。
use crate as update;
use crate::error::Reason;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};
mod candidate;
mod helper;
mod startup;
mod transaction;
mod workspace;
use candidate::{PackageKind, extract_package_zip, prepare};
use helper::spawn_helper;
pub use helper::{
    HelperLaunch, HelperRequest, HelperResult, HelperStarted, validate_helper_request,
};
pub use startup::{StartupUpdateResult, complete_startup_transaction};
pub use workspace::InstallTarget;
use workspace::InstallWorkspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStage {
    Preparing,
    Extracting,
    ApplyingIncremental,
    ApplyingFull,
    CleaningUp,
}

fn emit_install_stage(on_stage: &impl Fn(InstallStage), stage: InstallStage) {
    debug!(operation = "install", stage = ?stage, "更新安装进入新阶段");
    on_stage(stage);
}

fn validate_download_package(
    target: &InstallTarget,
    package_path: &Path,
) -> Result<PathBuf, String> {
    let downloads = target.root().join("cache/downloads");
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

/// 准备更新并把已发布的事务交给显式配置的 helper。
pub(crate) fn install_update_inner(
    target: &InstallTarget,
    helper_launch: impl FnOnce() -> Result<HelperLaunch, String>,
    package_path: &Path,
    on_stage: &impl Fn(InstallStage),
) -> Result<HelperStarted, update::Error> {
    emit_install_stage(on_stage, InstallStage::Preparing);
    let package_zip = validate_download_package(target, package_path)
        .map_err(|error| update::Error::failed(Reason::InvalidPackage, anyhow::anyhow!(error)))?;
    let workspace = InstallWorkspace::new(target);
    debug!(
        operation = "install",
        package = %package_zip.display(),
        workspace = %target.root().join("cache").join("update").display(),
        "更新安装路径校验完成"
    );
    // 没有正式 transaction 时，这些只能是未发布的准备残留，可以安全抛弃。新的
    // 安装绝不复用已经消费过的 ZIP 或半途 candidate。
    if let Err(error) = transaction::ensure_inactive(target) {
        let message = format!("开始安装前检查更新事务失败: {error}");
        return match fs::remove_file(&package_zip) {
            Ok(()) => Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(message),
            )),
            Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => Err(
                update::Error::failed(Reason::Preparation, anyhow::anyhow!(message)),
            ),
            Err(remove_error) => Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(
                    "{message}；删除下载文件 [{}] 也失败: {remove_error}",
                    package_zip.display()
                ),
            )),
        };
    }
    if let Err(error) = workspace.cleanup_inactive_material() {
        let message = format!("开始安装前清理旧更新文件失败: {error}");
        return match fs::remove_file(&package_zip) {
            Ok(()) => Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(message),
            )),
            Err(remove_error) if remove_error.kind() == std::io::ErrorKind::NotFound => Err(
                update::Error::failed(Reason::Preparation, anyhow::anyhow!(message)),
            ),
            Err(remove_error) => Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(
                    "{message}；删除下载文件 [{}] 也失败: {remove_error}",
                    package_zip.display()
                ),
            )),
        };
    }
    let package_dir = workspace.package_path();

    emit_install_stage(on_stage, InstallStage::Extracting);
    if let Err(error) = extract_package_zip(&package_zip, &package_dir) {
        return Err(update::Error::failed(
            Reason::Preparation,
            anyhow::anyhow!(cleanup_failed_preparation(target, &package_zip, error)),
        ));
    }

    let kind = PackageKind::detect(&package_dir);
    emit_install_stage(
        on_stage,
        match kind {
            PackageKind::Full => InstallStage::ApplyingFull,
            PackageKind::Incremental => InstallStage::ApplyingIncremental,
        },
    );
    let (prepared, _) = match prepare(target, &package_dir) {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(cleanup_failed_preparation(target, &package_zip, error)),
            ));
        }
    };
    let begun = transaction::begin(prepared).map_err(|error| {
        update::Error::failed(
            Reason::Preparation,
            anyhow::anyhow!(cleanup_failed_preparation(target, &package_zip, error)),
        )
    })?;

    // transaction 已证明 candidate 完整；zip 从此不能被复用，删除失败则不启动 helper。
    if let Err(error) = fs::remove_file(&package_zip) {
        let message = format!("删除已消费的更新包失败: {error}");
        return match begun.cancel() {
            Ok(()) => Err(update::Error::failed(
                Reason::Preparation,
                anyhow::anyhow!(message),
            )),
            Err(cleanup_error) => Err(update::Error::failed(
                Reason::FileAccess,
                anyhow::anyhow!("{message}；清理事务也失败: {cleanup_error}"),
            )),
        };
    }
    debug!(
        operation = "install",
        package = %package_zip.display(),
        "已删除完成消费的更新包"
    );
    emit_install_stage(on_stage, InstallStage::CleaningUp);
    if let Err(cleanup_error) = fs::remove_dir_all(&package_dir)
        && cleanup_error.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            operation = "install",
            path = %package_dir.display(),
            error = %cleanup_error,
            "清理已解压更新包失败，后端继续交接更新事务"
        );
    }

    let helper = match helper_launch().and_then(|launch| spawn_helper(target, &launch)) {
        Ok(helper) => helper,
        Err(error) => {
            return match begun.cancel() {
                Ok(()) => Err(update::Error::failed(
                    Reason::HelperStart,
                    anyhow::anyhow!(error),
                )),
                Err(cleanup) => Err(update::Error::failed(
                    Reason::HelperStart,
                    anyhow::anyhow!("{error}；清理事务也失败: {cleanup}"),
                )),
            };
        }
    };
    debug!(
        operation = "install",
        helper_pid = helper.id(),
        "更新 helper 已启动，当前进程准备退出"
    );
    info!("更新文件准备完成，安装程序已启动");

    // helper 已经独立接管事务；宿主应请求退出，释放 Windows 对根 exe 的占用。
    Ok(HelperStarted {
        process_id: helper.id(),
    })
}

fn cleanup_failed_preparation(
    target: &InstallTarget,
    package_zip: &Path,
    primary_error: String,
) -> String {
    let mut cleanup_errors = Vec::new();
    if let Err(error) = fs::remove_file(package_zip)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        cleanup_errors.push(format!(
            "删除下载文件 [{}] 失败: {error}",
            package_zip.display()
        ));
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
