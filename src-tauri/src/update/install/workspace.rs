//! 安装目录的固定布局。
//!
//! 这里不解释事务阶段；它只把 `InstallTarget` 转换为 candidate 与 transaction
//! 所需的受限站点。事务标记的读写和锁生命周期由 `transaction` 独占。

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::app_paths::AppPaths;

/// 一次安装所针对的应用 payload。
#[derive(Debug, Clone)]
pub(crate) struct InstallTarget {
    app_paths: AppPaths,
    executable_name: OsString,
}

impl InstallTarget {
    pub(crate) fn with_executable_name(
        app_paths: &AppPaths,
        executable_name: impl Into<OsString>,
    ) -> Self {
        Self {
            app_paths: app_paths.clone(),
            executable_name: executable_name.into(),
        }
    }

    pub(crate) fn for_current_executable(app_paths: &AppPaths) -> std::io::Result<Self> {
        let executable_name = if cfg!(target_os = "windows") {
            OsString::from("OEA.exe")
        } else {
            std::env::current_exe()?
                .file_name()
                .map(OsString::from)
                .ok_or_else(|| std::io::Error::other("当前 exe 没有文件名"))?
        };
        Ok(Self::with_executable_name(app_paths, executable_name))
    }

    pub(crate) fn root(&self) -> &Path {
        self.app_paths.root_dir()
    }
    pub(crate) fn executable_name(&self) -> &OsString {
        &self.executable_name
    }
    pub(crate) fn app_paths(&self) -> &AppPaths {
        &self.app_paths
    }
    pub(crate) fn prepare_helper_copy(&self, source: &Path) -> Result<PathBuf, String> {
        InstallWorkspace::new(self).prepare_helper_copy(source)
    }
    pub(crate) fn helper_path(&self) -> PathBuf {
        InstallWorkspace::new(self).helper_path()
    }
}

#[derive(Debug, Clone)]
pub(super) struct InstallWorkspace {
    target: InstallTarget,
}

impl InstallWorkspace {
    pub(super) fn new(target: &InstallTarget) -> Self {
        Self {
            target: target.clone(),
        }
    }

    pub(super) fn candidate_site(&self) -> CandidateSite {
        let update = self.update_path();
        CandidateSite {
            baseline: update.join("baseline"),
            candidate: update.join("candidate"),
            discard: update.join("discard"),
            update,
            executable: self.executable_path(),
            executable_name: self.target.executable_name().clone(),
            resources: self.target.app_paths.resources_dir(),
        }
    }

    pub(super) fn transaction_site(&self) -> TransactionSite {
        let update = self.update_path();
        TransactionSite {
            transaction: update.join("transaction.json"),
            lock: update.join("transaction.lock"),
            candidate: update.join("candidate"),
            discard: update.join("discard"),
            update,
            executable: self.executable_path(),
            executable_name: self.target.executable_name().clone(),
            resources: self.target.app_paths.resources_dir(),
        }
    }

    pub(super) fn package_path(&self) -> PathBuf {
        self.update_path().join("package")
    }

    pub(super) fn cleanup_inactive_material(&self) -> Result<(), String> {
        for path in [
            self.update_path().join("baseline"),
            self.update_path().join("candidate"),
            self.update_path()
                .join(format!("candidate.building-{}", std::process::id())),
            self.update_path().join("discard"),
            self.package_path(),
        ] {
            remove_directory_if_present(&path)?;
        }
        Ok(())
    }

    pub(super) fn best_effort_cleanup(&self) {
        for path in [
            self.update_path().join("baseline"),
            self.update_path().join("candidate"),
            self.update_path().join("discard"),
            self.update_path()
                .join(format!("candidate.building-{}", std::process::id())),
            self.package_path(),
            self.target.app_paths.cache_dir().join("old"),
            self.target.app_paths.cache_dir().join("update_extract"),
        ] {
            if let Err(error) = fs::remove_dir_all(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("清理更新目录 [{}] 失败: {error}", path.display());
                }
            }
        }
        if let Err(error) = fs::remove_file(self.helper_path()) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    "清理 helper 副本 [{}] 失败: {error}",
                    self.helper_path().display()
                );
            }
        }
    }

    fn executable_path(&self) -> PathBuf {
        self.target.root().join(self.target.executable_name())
    }
    fn update_path(&self) -> PathBuf {
        self.target.app_paths.cache_dir().join("update")
    }
    fn helper_path(&self) -> PathBuf {
        self.update_path().join(if cfg!(target_os = "windows") {
            "helper.exe"
        } else {
            "helper"
        })
    }

    fn prepare_helper_copy(&self, source: &Path) -> Result<PathBuf, String> {
        let metadata = fs::symlink_metadata(source)
            .map_err(|error| format!("读取当前 exe 失败 [{}]: {error}", source.display()))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("当前 exe 不是普通文件: {}", source.display()));
        }
        fs::create_dir_all(self.update_path())
            .map_err(|error| format!("创建 helper 工作区失败: {error}"))?;
        let helper = self.helper_path();
        let building = self
            .update_path()
            .join(format!("helper.building-{}", std::process::id()));
        remove_file_if_present(&building).map_err(|error| {
            format!(
                "清理旧 helper 临时文件失败 [{}]: {error}",
                building.display()
            )
        })?;
        remove_file_if_present(&helper)
            .map_err(|error| format!("清理旧 helper 副本失败 [{}]: {error}", helper.display()))?;
        fs::copy(source, &building).map_err(|error| {
            format!(
                "复制当前 exe 到 helper 副本失败 [{}] -> [{}]: {error}",
                source.display(),
                building.display()
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &building,
                fs::Permissions::from_mode(metadata.permissions().mode()),
            )
            .map_err(|error| format!("保留 helper 可执行权限失败: {error}"))?;
        }
        fs::rename(&building, &helper).map_err(|error| {
            format!(
                "发布 helper 副本失败 [{}] -> [{}]: {error}",
                building.display(),
                helper.display()
            )
        })?;
        Ok(helper)
    }
}

#[derive(Debug, Clone)]
pub(super) struct CandidateSite {
    pub(super) update: PathBuf,
    pub(super) baseline: PathBuf,
    pub(super) candidate: PathBuf,
    pub(super) discard: PathBuf,
    pub(super) executable: PathBuf,
    pub(super) executable_name: OsString,
    pub(super) resources: PathBuf,
}
impl CandidateSite {
    pub(super) fn building_path(&self) -> PathBuf {
        self.update
            .join(format!("candidate.building-{}", std::process::id()))
    }
}

#[derive(Debug, Clone)]
pub(super) struct TransactionSite {
    pub(super) update: PathBuf,
    pub(super) transaction: PathBuf,
    pub(super) lock: PathBuf,
    pub(super) candidate: PathBuf,
    pub(super) discard: PathBuf,
    pub(super) executable: PathBuf,
    pub(super) executable_name: OsString,
    pub(super) resources: PathBuf,
}

fn remove_file_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
pub(super) fn remove_directory_if_present(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("清理目录 [{}] 失败: {error}", path.display()))?;
    }
    Ok(())
}
