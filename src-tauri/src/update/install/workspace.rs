//! 更新工作区的固定布局与事务文件。
//!
//! `transaction.json` 是事务生命周期的唯一发布标记。候选目录必须先完整构造，
//! 然后才写入一个临时 transaction 文件并用 rename 发布；因此看到正式的
//! `transaction.json` 时，`candidate` 已经包含完整的 exe 与 resources。

use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::platform::update::FileLock;

/// 当前 transaction 文件格式版本。
pub const TRANSACTION_SCHEMA_VERSION: u32 = 1;

/// 固定的更新工作区。
#[derive(Debug, Clone)]
pub struct UpdateWorkspace {
    root: PathBuf,
    executable_name: OsString,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransactionFile {
    schema_version: u32,
}

impl UpdateWorkspace {
    /// 为测试或调用方提供显式的应用根目录和 exe 名称。
    pub fn with_executable_name(
        root: impl Into<PathBuf>,
        executable_name: impl Into<OsString>,
    ) -> Self {
        Self {
            root: root.into(),
            executable_name: executable_name.into(),
        }
    }

    /// 使用当前平台运行中的 exe 名称构造生产工作区。
    pub fn for_current_executable(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let executable_name = if cfg!(target_os = "windows") {
            OsString::from("OEA.exe")
        } else {
            std::env::current_exe()?
                .file_name()
                .map(OsString::from)
                .ok_or_else(|| std::io::Error::other("当前 exe 没有文件名"))?
        };
        Ok(Self::with_executable_name(root, executable_name))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn executable_name(&self) -> &OsString {
        &self.executable_name
    }

    pub fn executable_path(&self) -> PathBuf {
        self.root.join(&self.executable_name)
    }

    pub fn resources_path(&self) -> PathBuf {
        self.root.join("resources")
    }

    pub fn cache_path(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn update_path(&self) -> PathBuf {
        self.cache_path().join("update")
    }

    pub fn transaction_path(&self) -> PathBuf {
        self.update_path().join("transaction.json")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.update_path().join("transaction.lock")
    }

    pub fn baseline_path(&self) -> PathBuf {
        self.update_path().join("baseline")
    }

    pub fn candidate_path(&self) -> PathBuf {
        self.update_path().join("candidate")
    }

    pub fn discard_path(&self) -> PathBuf {
        self.update_path().join("discard")
    }

    /// ZIP 解压后的临时 package 目录，不属于 transaction 状态。
    pub fn package_path(&self) -> PathBuf {
        self.update_path().join("package")
    }

    /// helper 的可执行副本路径。
    ///
    /// helper 不能直接执行根目录下当前 exe，因为 Windows 可能仍持有该文件，且
    /// helper 正要替换它。副本由安装命令创建，下一次启动时尽力清理。
    pub fn helper_path(&self) -> PathBuf {
        self.update_path().join(if cfg!(target_os = "windows") {
            "helper.exe"
        } else {
            "helper"
        })
    }

    pub fn candidate_build_path(&self) -> PathBuf {
        self.update_path()
            .join(format!("candidate.building-{}", std::process::id()))
    }

    /// 把当前正在运行的 exe 复制为 helper 副本，并保留 Unix 可执行权限。
    ///
    /// 成功前正式 helper 路径不变；成功后 helper 副本是一个独立文件，可以在旧 exe
    /// 退出后运行。若复制失败，根目录 exe/resources 和事务文件都不受影响。
    pub fn prepare_helper_copy(&self, source: &Path) -> Result<PathBuf, String> {
        let metadata = fs::symlink_metadata(source)
            .map_err(|error| format!("读取当前 exe 失败 [{}]: {error}", source.display()))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!("当前 exe 不是普通文件: {}", source.display()));
        }
        self.ensure_update_path()
            .map_err(|error| format!("创建 helper 工作区失败: {error}"))?;

        let helper = self.helper_path();
        let building = self
            .update_path()
            .join(format!("helper.building-{}", std::process::id()));
        let _ = fs::remove_file(&building);
        let _ = fs::remove_file(&helper);
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

    pub fn ensure_update_path(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.update_path())
    }

    /// 尝试获得 transaction lock。
    pub fn try_lock(&self) -> std::io::Result<Option<FileLock>> {
        FileLock::try_acquire(&self.lock_path())
    }

    /// 阻塞直到获得 transaction lock。helper 使用它等待 v1 释放事务权限。
    pub fn lock(&self) -> std::io::Result<FileLock> {
        FileLock::acquire(&self.lock_path())
    }

    pub fn transaction_exists(&self) -> bool {
        self.transaction_path().is_file()
    }

    /// 读取并验证 transaction 文件，拒绝未知 schema。
    pub fn read_transaction(&self) -> Result<Option<()>, String> {
        if !self.transaction_exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(self.transaction_path())
            .map_err(|error| format!("读取 transaction.json 失败: {error}"))?;
        let transaction: TransactionFile = serde_json::from_str(&content)
            .map_err(|error| format!("解析 transaction.json 失败: {error}"))?;
        if transaction.schema_version != TRANSACTION_SCHEMA_VERSION {
            return Err(format!(
                "不支持的 transaction schema version: {}",
                transaction.schema_version
            ));
        }
        Ok(Some(()))
    }

    /// 原子发布 transaction 文件。
    ///
    /// 调用前必须已完成并发布 `candidate`。成功前正式 transaction 不存在；成功后
    /// transaction 的存在证明 candidate 构造成功。若写入或 rename 失败，临时文件会
    /// 尽力删除，调用方可以清理 candidate 并保持应用根目录不变。
    pub fn publish_transaction(&self) -> Result<(), String> {
        if self.transaction_exists() {
            return Err("transaction.json 已存在，不能开始第二个更新事务".to_string());
        }
        self.ensure_update_path()
            .map_err(|error| format!("创建更新工作区失败: {error}"))?;

        let temporary = self
            .update_path()
            .join(format!("transaction.json.tmp-{}", std::process::id()));
        let result = (|| {
            let transaction = TransactionFile {
                schema_version: TRANSACTION_SCHEMA_VERSION,
            };
            let content = serde_json::to_vec(&transaction)
                .map_err(|error| format!("序列化 transaction.json 失败: {error}"))?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| format!("创建 transaction 临时文件失败: {error}"))?;
            file.write_all(&content)
                .map_err(|error| format!("写入 transaction 临时文件失败: {error}"))?;
            file.write_all(b"\n")
                .map_err(|error| format!("写入 transaction 临时文件失败: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("刷新 transaction 临时文件失败: {error}"))?;
            fs::rename(&temporary, self.transaction_path())
                .map_err(|error| format!("原子发布 transaction.json 失败: {error}"))
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// 删除 transaction 标记。只有资源已经提交且 candidate 为空时调用。
    pub fn remove_transaction(&self) -> Result<(), String> {
        match fs::remove_file(self.transaction_path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("删除 transaction.json 失败: {error}")),
        }
    }

    /// 清理一次失败的准备或 helper exe 替换。
    ///
    /// 这个操作只删除 cache/update 内的事务材料，不触碰根目录的 exe/resources。
    pub fn remove_transaction_workspace(&self) -> Result<(), String> {
        self.remove_transaction()?;
        for path in [
            self.baseline_path(),
            self.candidate_path(),
            self.candidate_build_path(),
            self.discard_path(),
            self.package_path(),
        ] {
            if path.exists() {
                fs::remove_dir_all(&path)
                    .map_err(|error| format!("清理更新目录 [{}] 失败: {error}", path.display()))?;
            }
        }
        Ok(())
    }

    /// 成功提交或确认没有事务后，尽力清理不再参与事务的目录。
    ///
    /// `cache/old` 与 `cache/update_extract` 来自旧安装器。它们不参与新事务，可以在
    /// 没有 transaction 时安全删除。`cache/config_backup` 是用户既有数据，刻意保留。
    pub fn best_effort_cleanup(&self) {
        for path in [
            self.baseline_path(),
            self.candidate_path(),
            self.discard_path(),
            self.candidate_build_path(),
            self.package_path(),
            self.cache_path().join("old"),
            self.cache_path().join("update_extract"),
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
}
