//! 活跃更新事务的唯一解释者。
//!
//! `transaction.json` 只表示活跃事务和 `schema version`。持锁后由 `marker` 与 `payload`
//! 文件树重新构造私有阶段，三个进程角色只通过各自的入口推进事务。

use std::{
    fs::{self, OpenOptions},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::platform::{file::replace, update::FileLock};

use super::{
    candidate::PreparedCandidate,
    helper::HelperResult,
    startup::StartupUpdateResult,
    workspace::{InstallTarget, InstallWorkspace, TransactionSite},
};

const TRANSACTION_SCHEMA_VERSION: u32 = 1;
const REPLACE_RETRY_WINDOW: Duration = Duration::from_secs(30);
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransactionFile {
    schema_version: u32,
}

#[derive(Debug)]
enum TransactionError {
    CandidateIncomplete,
    Lock {
        operation: &'static str,
        source: io::Error,
    },
    DirectoryCreate {
        directory: &'static str,
        source: io::Error,
    },
    MarkerRead {
        source: io::Error,
    },
    MarkerParse {
        source: serde_json::Error,
    },
    UnsupportedSchema {
        found: u32,
    },
    MarkerWrite {
        operation: &'static str,
        source: io::Error,
    },
    MarkerEncode {
        source: serde_json::Error,
    },
    InvalidLayout {
        detail: &'static str,
    },
    PayloadMove {
        operation: &'static str,
        source: PathBuf,
        target: PathBuf,
        error: io::Error,
    },
    CandidateRead {
        source: io::Error,
    },
    CandidateNotEmpty,
    CleanupDirectory {
        path: PathBuf,
        source: io::Error,
    },
    ExecutableReplace {
        source: io::Error,
    },
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CandidateIncomplete => f.write_str("candidate 不完整，不能开始更新事务"),
            Self::Lock { operation, source } => {
                write!(f, "{operation} transaction.lock 失败: {source}")
            }
            Self::DirectoryCreate { directory, source } => {
                write!(f, "创建 {directory} 目录失败: {source}")
            }
            Self::MarkerRead { source } => write!(f, "读取 transaction.json 失败: {source}"),
            Self::MarkerParse { source } => write!(f, "解析 transaction.json 失败: {source}"),
            Self::UnsupportedSchema { found } => {
                write!(f, "不支持的 transaction schema version: {found}")
            }
            Self::MarkerWrite { operation, source } => {
                write!(f, "{operation} transaction.json 失败: {source}")
            }
            Self::MarkerEncode { source } => write!(f, "序列化 transaction.json 失败: {source}"),
            Self::InvalidLayout { detail } => f.write_str(detail),
            Self::PayloadMove {
                operation,
                source,
                target,
                error,
            } => {
                write!(
                    f,
                    "{operation} [{}] -> [{}] 失败: {error}",
                    source.display(),
                    target.display()
                )
            }
            Self::CandidateRead { source } => write!(f, "读取 candidate 状态失败: {source}"),
            Self::CandidateNotEmpty => f.write_str("resources 已提交但 candidate 仍有未预期的条目"),
            Self::CleanupDirectory { path, source } => {
                write!(f, "清理更新目录 [{}] 失败: {source}", path.display())
            }
            Self::ExecutableReplace { source } => write!(f, "替换应用 exe 失败: {source}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TransactionState {
    Inactive,
    Active(ActiveState),
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)] // 名称直接对应持久化操作完成前的四个事务阶段。
enum ActiveState {
    AwaitingExecutableCommit,
    AwaitingOldResourcesMove,
    AwaitingNewResourcesMove,
    AwaitingTransactionRemoval,
}

/// v1 已开始事务的能力。它只允许在 `helper` 启动前显式取消。
pub(crate) struct BegunTransaction {
    target: InstallTarget,
}

impl BegunTransaction {
    pub(crate) fn cancel(self) -> Result<(), String> {
        let mut transaction = LockedTransaction::acquire(&self.target)?;
        if matches!(
            transaction.state,
            TransactionState::Active(ActiveState::AwaitingExecutableCommit)
        ) {
            transaction.cancel()?;
        }
        Ok(())
    }
}

/// 在 `candidate` 发布后开始事务并原子发布 `marker`。
pub(crate) fn begin(candidate: PreparedCandidate) -> Result<BegunTransaction, String> {
    let target = candidate.into_target();
    let mut transaction = LockedTransaction::acquire(&target)?;
    if !matches!(transaction.state, TransactionState::Inactive) {
        return Err("transaction.json 已存在，不能开始第二个更新事务".to_string());
    }
    transaction.validate_prepared_candidate()?;
    transaction.publish_marker()?;
    transaction.state = TransactionState::Active(ActiveState::AwaitingExecutableCommit);
    debug!(operation = "install", "更新事务标记已发布");
    Ok(BegunTransaction { target })
}

/// 确认不存在活跃 `marker`。调用方在返回后负责清理准备残留。
pub(crate) fn ensure_inactive(target: &InstallTarget) -> Result<(), String> {
    let transaction = LockedTransaction::acquire(target)?;
    if !matches!(transaction.state, TransactionState::Inactive) {
        return Err("已有更新事务正在处理；请稍后重新下载".to_string());
    }
    Ok(())
}

/// `helper` 专用的 `executable` 提交入口。
pub(crate) fn commit_executable(
    target: &InstallTarget,
    on_commit_start: impl FnOnce(),
) -> Result<HelperResult, String> {
    let mut transaction = LockedTransaction::acquire(target)?;
    match transaction.state {
        TransactionState::Inactive => return Ok(HelperResult::NoTransaction),
        TransactionState::Active(ActiveState::AwaitingExecutableCommit) => {}
        TransactionState::Active(_) => return Ok(HelperResult::AlreadyCommitted),
    }

    let candidate = transaction
        .site
        .candidate
        .join(&transaction.site.executable_name);
    on_commit_start();
    let started_at = Instant::now();
    let deadline = started_at + REPLACE_RETRY_WINDOW;
    let mut attempts = 0u32;
    let last_error = loop {
        attempts += 1;
        match replace(&candidate, &transaction.site.executable) {
            Ok(()) => {
                transaction.state = TransactionState::Active(ActiveState::AwaitingOldResourcesMove);
                debug!(
                    process_role = "update_helper",
                    attempts,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "更新 helper 已提交 executable"
                );
                return Ok(HelperResult::ExecutableCommitted);
            }
            Err(error) if Instant::now() < deadline => {
                if attempts == 1 {
                    debug!(process_role = "update_helper", error = %error, "executable 暂时无法替换，等待旧进程释放文件");
                }
                thread::sleep(REPLACE_RETRY_DELAY);
            }
            Err(error) => break error,
        }
    };
    let message = TransactionError::ExecutableReplace { source: last_error }.to_string();
    match transaction.cancel() {
        Ok(()) => Err(message),
        Err(cleanup) => Err(format!("{message}；清理事务也失败: {cleanup}")),
    }
}

/// v2 启动专用的 `resources` 完成入口。
pub(crate) fn complete_startup(
    target: &InstallTarget,
    on_commit_start: impl FnOnce(),
) -> Result<StartupUpdateResult, String> {
    let workspace = InstallWorkspace::new(target);
    if !workspace.has_transaction_marker() {
        return Ok(StartupUpdateResult::NoTransaction);
    }
    let Some(mut transaction) = LockedTransaction::try_acquire(target)? else {
        return Err("更新事务正在由另一个进程处理".to_string());
    };
    match transaction.state {
        TransactionState::Inactive => return Ok(StartupUpdateResult::NoTransaction),
        TransactionState::Active(ActiveState::AwaitingExecutableCommit) => {
            return Ok(StartupUpdateResult::WaitingForHelper);
        }
        TransactionState::Active(_) => {}
    }

    if matches!(
        transaction.state,
        TransactionState::Active(
            ActiveState::AwaitingOldResourcesMove | ActiveState::AwaitingNewResourcesMove
        )
    ) {
        on_commit_start();
    }
    transaction.complete_resources()?;
    transaction
        .remove_marker()
        .map_err(|error| format!("资源已提交，但删除 transaction 失败: {error}"))?;
    transaction.state = TransactionState::Inactive;
    debug!(
        process_role = "app_startup",
        result = "completed",
        "更新事务提交完成，resources 已切换到新版本"
    );
    Ok(StartupUpdateResult::Completed)
}

/// 私有 RAII guard。析构只释放 `FileLock`，不会悄悄改变任何事务文件。
struct LockedTransaction {
    _lock: FileLock,
    site: TransactionSite,
    state: TransactionState,
}

impl LockedTransaction {
    fn acquire(target: &InstallTarget) -> Result<Self, String> {
        let site = InstallWorkspace::new(target).transaction_site();
        let lock = FileLock::acquire(&site.lock).map_err(|source| {
            TransactionError::Lock {
                operation: "获取",
                source,
            }
            .to_string()
        })?;
        Self::from_locked(site, lock)
    }

    fn try_acquire(target: &InstallTarget) -> Result<Option<Self>, String> {
        let site = InstallWorkspace::new(target).transaction_site();
        let Some(lock) = FileLock::try_acquire(&site.lock).map_err(|source| {
            TransactionError::Lock {
                operation: "打开",
                source,
            }
            .to_string()
        })?
        else {
            return Ok(None);
        };
        Self::from_locked(site, lock).map(Some)
    }

    fn from_locked(site: TransactionSite, lock: FileLock) -> Result<Self, String> {
        let state = infer_state(&site)?;
        Ok(Self {
            _lock: lock,
            site,
            state,
        })
    }

    fn validate_prepared_candidate(&self) -> Result<(), String> {
        let executable = self.site.candidate.join(&self.site.executable_name);
        let resources = self.site.candidate.join("resources");
        if !executable.is_file() || !resources.is_dir() {
            return Err(TransactionError::CandidateIncomplete.to_string());
        }
        Ok(())
    }

    fn publish_marker(&self) -> Result<(), String> {
        fs::create_dir_all(&self.site.update).map_err(|source| {
            TransactionError::DirectoryCreate {
                directory: "更新工作区",
                source,
            }
            .to_string()
        })?;
        let temporary = self
            .site
            .update
            .join(format!("transaction.json.tmp-{}", std::process::id()));
        let result = (|| {
            let content = serde_json::to_vec(&TransactionFile {
                schema_version: TRANSACTION_SCHEMA_VERSION,
            })
            .map_err(|source| TransactionError::MarkerEncode { source })?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|source| TransactionError::MarkerWrite {
                    operation: "创建",
                    source,
                })?;
            file.write_all(&content)
                .map_err(|source| TransactionError::MarkerWrite {
                    operation: "写入",
                    source,
                })?;
            file.write_all(b"\n")
                .map_err(|source| TransactionError::MarkerWrite {
                    operation: "写入",
                    source,
                })?;
            file.sync_all()
                .map_err(|source| TransactionError::MarkerWrite {
                    operation: "刷新",
                    source,
                })?;
            fs::rename(&temporary, &self.site.transaction).map_err(|source| {
                TransactionError::MarkerWrite {
                    operation: "原子发布",
                    source,
                }
            })
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => match remove_file_if_present(&temporary) {
                Ok(()) => Err(error.to_string()),
                Err(cleanup) => Err(format!(
                    "{error}；清理 transaction 临时文件 [{}] 也失败: {cleanup}",
                    temporary.display()
                )),
            },
        }
    }

    fn complete_resources(&mut self) -> Result<(), String> {
        if matches!(
            self.state,
            TransactionState::Active(ActiveState::AwaitingOldResourcesMove)
        ) {
            let discard_resources = self.site.discard.join("resources");
            fs::create_dir_all(&self.site.discard).map_err(|source| {
                TransactionError::DirectoryCreate {
                    directory: "discard",
                    source,
                }
                .to_string()
            })?;
            fs::rename(&self.site.resources, &discard_resources).map_err(|error| {
                TransactionError::PayloadMove {
                    operation: "原子移动旧 resources",
                    source: self.site.resources.clone(),
                    target: discard_resources.clone(),
                    error,
                }
                .to_string()
            })?;
            self.state = TransactionState::Active(ActiveState::AwaitingNewResourcesMove);
        }
        if matches!(
            self.state,
            TransactionState::Active(ActiveState::AwaitingNewResourcesMove)
        ) {
            let candidate_resources = self.site.candidate.join("resources");
            fs::rename(&candidate_resources, &self.site.resources).map_err(|error| {
                TransactionError::PayloadMove {
                    operation: "原子移动 candidate resources",
                    source: candidate_resources.clone(),
                    target: self.site.resources.clone(),
                    error,
                }
                .to_string()
            })?;
            self.state = TransactionState::Active(ActiveState::AwaitingTransactionRemoval);
        }
        if !matches!(
            self.state,
            TransactionState::Active(ActiveState::AwaitingTransactionRemoval)
        ) {
            return Err(TransactionError::InvalidLayout {
                detail: "资源事务状态不允许完成",
            }
            .to_string());
        }
        self.remove_empty_candidate()?;
        Ok(())
    }

    fn remove_empty_candidate(&self) -> Result<(), String> {
        if self.site.candidate.exists() {
            let mut entries = fs::read_dir(&self.site.candidate)
                .map_err(|source| TransactionError::CandidateRead { source }.to_string())?;
            if entries.next().is_some() {
                return Err(TransactionError::CandidateNotEmpty.to_string());
            }
            if let Err(error) = fs::remove_dir(&self.site.candidate)
                && error.kind() != ErrorKind::NotFound
            {
                warn!("清理空 candidate 目录失败: {error}");
            }
        }
        Ok(())
    }

    fn remove_marker(&self) -> Result<(), String> {
        remove_file_if_present(&self.site.transaction).map_err(|source| {
            TransactionError::MarkerWrite {
                operation: "删除",
                source,
            }
            .to_string()
        })
    }

    fn cancel(&mut self) -> Result<(), String> {
        self.remove_marker()?;
        for path in [
            self.site.update.join("baseline"),
            self.site.candidate.clone(),
            self.site.discard.clone(),
            self.site
                .update
                .join(format!("candidate.building-{}", std::process::id())),
            self.site.update.join("package"),
        ] {
            if path.exists() {
                fs::remove_dir_all(&path).map_err(|source| {
                    TransactionError::CleanupDirectory {
                        path: path.clone(),
                        source,
                    }
                    .to_string()
                })?;
            }
        }
        self.state = TransactionState::Inactive;
        Ok(())
    }
}

fn infer_state(site: &TransactionSite) -> Result<TransactionState, String> {
    if !site.transaction.is_file() {
        return Ok(TransactionState::Inactive);
    }
    let content = fs::read_to_string(&site.transaction)
        .map_err(|source| TransactionError::MarkerRead { source }.to_string())?;
    let marker: TransactionFile = serde_json::from_str(&content)
        .map_err(|source| TransactionError::MarkerParse { source }.to_string())?;
    if marker.schema_version != TRANSACTION_SCHEMA_VERSION {
        return Err(TransactionError::UnsupportedSchema {
            found: marker.schema_version,
        }
        .to_string());
    }
    let candidate_executable = site.candidate.join(&site.executable_name);
    let candidate_resources = site.candidate.join("resources");
    let discard_resources = site.discard.join("resources");
    let state = if candidate_executable.exists() {
        ActiveState::AwaitingExecutableCommit
    } else if candidate_resources.is_dir() && site.resources.is_dir() && !discard_resources.exists()
    {
        ActiveState::AwaitingOldResourcesMove
    } else if candidate_resources.is_dir() && !site.resources.exists() && discard_resources.is_dir()
    {
        ActiveState::AwaitingNewResourcesMove
    } else if !candidate_resources.exists() && site.resources.is_dir() {
        ActiveState::AwaitingTransactionRemoval
    } else {
        return Err(TransactionError::InvalidLayout {
            detail: "transaction 文件树状态不一致，无法恢复",
        }
        .to_string());
    };
    Ok(TransactionState::Active(state))
}

fn remove_file_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
