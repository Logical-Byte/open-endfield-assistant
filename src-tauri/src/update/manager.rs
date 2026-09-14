use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Result, bail};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::check::AvailableUpdateMetadata;

/// 集中管理更新下载和安装的后端状态。
pub struct UpdateManager {
    inner: Mutex<UpdateState>,
}

struct UpdateState {
    next_session_id: u64,
    available_update: Option<AvailableUpdateMetadata>,
    pending_update: Option<PendingUpdate>,
    operation: UpdateOperation,
}

struct PendingUpdate {
    package_path: PathBuf,
    info: UpdateInfo,
}

enum UpdateOperation {
    Idle,
    Checking,
    Downloading(Arc<DownloadSession>),
    Installing,
}

impl UpdateOperation {
    fn status(&self) -> UpdateOperationStatus {
        match self {
            Self::Idle => UpdateOperationStatus::Idle,
            Self::Checking => UpdateOperationStatus::Checking,
            Self::Downloading(_) => UpdateOperationStatus::Downloading,
            Self::Installing => UpdateOperationStatus::Installing,
        }
    }
}

/// 前端可见的更新展示信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub(super) version_name: String,
    pub(super) release_note: String,
}

/// 前端可见的更新操作状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateOperationStatus {
    Idle,
    Checking,
    Downloading,
    Installing,
}

/// 一次加锁取得的更新状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    operation: UpdateOperationStatus,
    available_update: Option<UpdateInfo>,
    pending_update: Option<UpdateInfo>,
}

pub(super) struct CheckLease<'a> {
    manager: &'a UpdateManager,
    completed: bool,
}

pub(super) struct DownloadSession {
    id: u64,
    cancellation: CancellationToken,
    downloaded_bytes: AtomicU64,
}

pub(super) struct DownloadLease<'a> {
    manager: &'a UpdateManager,
    session: Arc<DownloadSession>,
    available_update: AvailableUpdateMetadata,
    completed: bool,
}

pub(super) struct InstallLease<'a> {
    manager: &'a UpdateManager,
    pending_update: PendingUpdate,
    completed: bool,
}

pub(super) struct DeveloperInstallLease<'a> {
    manager: &'a UpdateManager,
    completed: bool,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self {
            inner: Mutex::new(UpdateState {
                next_session_id: 0,
                available_update: None,
                pending_update: None,
                operation: UpdateOperation::Idle,
            }),
        }
    }
}

impl UpdateManager {
    pub(super) fn start_check(&self) -> Result<CheckLease<'_>> {
        let mut state = self.lock_state();
        if state.pending_update.is_some() {
            bail!("已有待安装更新，无法重新检查更新");
        }
        if !matches!(state.operation, UpdateOperation::Idle) {
            bail!("已有更新操作正在进行，请稍后重试");
        }
        state.available_update = None;
        state.operation = UpdateOperation::Checking;
        debug!(from = "idle", to = "checking", "更新状态机完成状态转换");
        Ok(CheckLease {
            manager: self,
            completed: false,
        })
    }

    /// 使用已缓存的可用更新开始一次完整下载操作。
    pub(super) fn start_update_download(&self) -> Result<DownloadLease<'_>> {
        let mut state = self.lock_state();
        if state.pending_update.is_some() {
            bail!("已有待安装更新，无法重复下载");
        }
        match state.operation {
            UpdateOperation::Idle => {}
            UpdateOperation::Checking => bail!("检查更新期间无法开始下载"),
            UpdateOperation::Downloading(_) => bail!("更新下载已在进行"),
            UpdateOperation::Installing => bail!("安装更新期间无法开始下载"),
        }
        let available_update = state
            .available_update
            .clone()
            .ok_or_else(|| anyhow::anyhow!("没有可用的更新，请先检查更新"))?;

        state.next_session_id += 1;
        let session = Arc::new(DownloadSession {
            id: state.next_session_id,
            cancellation: CancellationToken::new(),
            downloaded_bytes: AtomicU64::new(0),
        });
        state.operation = UpdateOperation::Downloading(Arc::clone(&session));
        debug!(
            from = "idle",
            to = "downloading",
            session_id = session.id,
            version = %available_update.version_name,
            "更新状态机完成状态转换"
        );

        Ok(DownloadLease {
            manager: self,
            session,
            available_update,
            completed: false,
        })
    }

    pub(super) fn cancel_download(&self) -> Result<()> {
        let state = self.lock_state();
        match &state.operation {
            UpdateOperation::Downloading(session) => {
                session.cancel();
                debug!(
                    operation = "download",
                    session_id = session.id,
                    downloaded_bytes = session.downloaded_bytes(),
                    "更新下载取消信号已设置"
                );
                info!("正在取消更新下载");
                Ok(())
            }
            UpdateOperation::Idle | UpdateOperation::Checking => {
                debug!(
                    operation = "cancel_download",
                    current_state = ?state.operation.status(),
                    "当前没有下载任务，取消请求无需处理"
                );
                Ok(())
            }
            UpdateOperation::Installing => bail!("安装更新期间无法取消下载"),
        }
    }

    /// 原子取走待安装更新并进入安装状态。
    pub(super) fn start_install(&self) -> Result<InstallLease<'_>> {
        let mut state = self.lock_state();
        match state.operation {
            UpdateOperation::Idle => {}
            UpdateOperation::Checking => bail!("检查更新期间无法开始安装更新"),
            UpdateOperation::Downloading(_) => bail!("下载期间无法开始安装更新"),
            UpdateOperation::Installing => bail!("更新安装已在进行"),
        }
        let pending_update = state
            .pending_update
            .take()
            .ok_or_else(|| anyhow::anyhow!("没有待安装的更新，请先下载更新"))?;
        state.operation = UpdateOperation::Installing;
        debug!(
            from = "idle",
            to = "installing",
            version = %pending_update.info.version_name,
            "更新状态机完成状态转换"
        );
        Ok(InstallLease {
            manager: self,
            pending_update,
            completed: false,
        })
    }

    /// 开发者本地包使用独立入口，不创建或消费普通待安装更新。
    pub(super) fn start_developer_install(&self) -> Result<DeveloperInstallLease<'_>> {
        let mut state = self.lock_state();
        if state.pending_update.is_some() {
            bail!("已有待安装更新，无法开始开发者安装");
        }
        match state.operation {
            UpdateOperation::Idle => {
                state.operation = UpdateOperation::Installing;
                debug!(
                    from = "idle",
                    to = "installing",
                    install_kind = "developer_package",
                    "更新状态机完成状态转换"
                );
            }
            UpdateOperation::Checking => bail!("检查更新期间无法开始安装更新"),
            UpdateOperation::Downloading(_) => bail!("下载期间无法开始安装更新"),
            UpdateOperation::Installing => bail!("更新安装已在进行"),
        }
        Ok(DeveloperInstallLease {
            manager: self,
            completed: false,
        })
    }

    /// 返回不包含安装包路径和下载会话的原子状态快照。
    pub fn status(&self) -> UpdateStatus {
        let state = self.lock_state();
        UpdateStatus {
            operation: state.operation.status(),
            available_update: state.available_update.as_ref().map(UpdateInfo::from),
            pending_update: state
                .pending_update
                .as_ref()
                .map(|pending| pending.info.clone()),
        }
    }

    pub(crate) fn is_installing(&self) -> bool {
        self.status().operation == UpdateOperationStatus::Installing
    }

    fn finish_check(&self, available_update: Option<AvailableUpdateMetadata>) {
        let mut state = self.lock_state();
        if matches!(state.operation, UpdateOperation::Checking) {
            let result = if available_update.is_some() {
                "available"
            } else {
                "no_update_or_failed"
            };
            state.available_update = available_update;
            state.operation = UpdateOperation::Idle;
            debug!(
                from = "checking",
                to = "idle",
                result,
                "更新状态机完成状态转换"
            );
        } else {
            warn!(
                expected_state = "checking",
                actual_state = ?state.operation.status(),
                "更新检查结束时状态机处于意外状态"
            );
        }
    }

    fn finish_download(&self, session_id: u64) {
        let mut state = self.lock_state();
        if matches!(&state.operation, UpdateOperation::Downloading(session) if session.id == session_id)
        {
            state.operation = UpdateOperation::Idle;
            debug!(
                from = "downloading",
                to = "idle",
                session_id,
                result = "incomplete",
                "更新状态机完成状态转换"
            );
        } else {
            warn!(
                expected_state = "downloading",
                actual_state = ?state.operation.status(),
                session_id,
                "更新下载结束时状态机或会话不匹配"
            );
        }
    }

    fn complete_download(
        &self,
        session_id: u64,
        package_path: PathBuf,
        metadata: &AvailableUpdateMetadata,
    ) {
        let mut state = self.lock_state();
        if matches!(&state.operation, UpdateOperation::Downloading(session) if session.id == session_id)
        {
            state.pending_update = Some(PendingUpdate {
                package_path,
                info: UpdateInfo::from(metadata),
            });
            state.operation = UpdateOperation::Idle;
            debug!(
                from = "downloading",
                to = "idle",
                session_id,
                result = "pending_install",
                version = %metadata.version_name,
                "更新状态机完成状态转换"
            );
        } else {
            warn!(
                expected_state = "downloading",
                actual_state = ?state.operation.status(),
                session_id,
                "登记待安装更新时状态机或会话不匹配"
            );
        }
    }

    fn finish_failed_install(&self) {
        let mut state = self.lock_state();
        if matches!(state.operation, UpdateOperation::Installing) {
            state.operation = UpdateOperation::Idle;
            debug!(
                from = "installing",
                to = "idle",
                result = "failed",
                "更新状态机完成状态转换；待安装包已消费，需要重新下载"
            );
        } else {
            warn!(
                expected_state = "installing",
                actual_state = ?state.operation.status(),
                "安装失败回滚时状态机处于意外状态"
            );
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, UpdateState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl From<&AvailableUpdateMetadata> for UpdateInfo {
    fn from(metadata: &AvailableUpdateMetadata) -> Self {
        Self {
            version_name: metadata.version_name.clone(),
            release_note: metadata.release_note.clone(),
        }
    }
}

impl CheckLease<'_> {
    pub(super) fn complete(mut self, available_update: Option<AvailableUpdateMetadata>) {
        self.manager.finish_check(available_update);
        self.completed = true;
    }
}

impl Drop for CheckLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.manager.finish_check(None);
        }
    }
}

impl DownloadLease<'_> {
    pub(super) fn id(&self) -> u64 {
        self.session.id
    }

    pub(super) fn session(&self) -> Arc<DownloadSession> {
        Arc::clone(&self.session)
    }

    pub(super) fn available_update(&self) -> &AvailableUpdateMetadata {
        &self.available_update
    }

    /// 发布成功后登记待安装更新。此后不再观察取消请求。
    pub(super) fn complete(mut self, package_path: PathBuf) {
        self.manager
            .complete_download(self.session.id, package_path, &self.available_update);
        self.completed = true;
    }
}

impl DownloadSession {
    fn cancel(&self) {
        self.cancellation.cancel();
    }

    #[cfg(test)]
    pub(super) fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub(super) fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub(super) fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }

    pub(super) fn set_downloaded_bytes(&self, bytes: u64) {
        self.downloaded_bytes.store(bytes, Ordering::Relaxed);
    }
}

impl Drop for DownloadLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            debug!(
                operation = "download",
                session_id = self.session.id,
                downloaded_bytes = self.session.downloaded_bytes(),
                cancelled = self.session.cancellation.is_cancelled(),
                "更新下载 lease 未完成，准备恢复空闲状态"
            );
            self.manager.finish_download(self.session.id);
        }
    }
}

impl InstallLease<'_> {
    pub(super) fn package_path(&self) -> &Path {
        &self.pending_update.package_path
    }

    /// helper 接管成功后保持 `Installing`，直到当前进程退出。
    pub(super) fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for InstallLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            warn!(
                operation = "install",
                version = %self.pending_update.info.version_name,
                "更新安装未交给 helper，准备恢复空闲状态"
            );
            self.manager.finish_failed_install();
        }
    }
}

impl DeveloperInstallLease<'_> {
    /// helper 接管成功后保持 `Installing`，直到当前进程退出。
    pub(super) fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for DeveloperInstallLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            debug!(
                operation = "install",
                install_kind = "developer_package",
                "开发者更新安装未开始或未交给 helper，准备恢复空闲状态"
            );
            self.manager.finish_failed_install();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{UpdateManager, UpdateOperationStatus};
    use crate::update::check::AvailableUpdateMetadata;

    fn metadata(version: &str) -> AvailableUpdateMetadata {
        AvailableUpdateMetadata {
            version_name: version.to_string(),
            release_note: "notes".to_string(),
            mirrorchyan_package: None,
        }
    }

    fn cache_update(manager: &UpdateManager) {
        manager
            .start_check()
            .unwrap()
            .complete(Some(metadata("1.3.0")));
    }

    fn complete_download(manager: &UpdateManager) {
        manager
            .start_update_download()
            .unwrap()
            .complete(PathBuf::from("C:/OEA/cache/downloads/update.zip"));
    }

    #[test]
    fn check_replaces_cache_and_all_exit_paths_restore_idle() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        assert_eq!(manager.status().operation, UpdateOperationStatus::Idle);

        let failed_check = manager.start_check().unwrap();
        assert!(manager.status().available_update.is_none());
        assert!(manager.start_update_download().is_err());
        assert!(manager.start_install().is_err());
        drop(failed_check);

        manager.start_check().unwrap().complete(None);
        assert!(manager.status().available_update.is_none());
        assert!(manager.start_update_download().is_err());
    }

    #[test]
    fn operation_leases_are_mutually_exclusive() {
        let manager = UpdateManager::default();
        manager.cancel_download().unwrap();

        let check = manager.start_check().unwrap();
        assert!(manager.start_check().is_err());
        assert!(manager.start_update_download().is_err());
        assert!(manager.start_install().is_err());
        assert!(manager.start_developer_install().is_err());
        manager.cancel_download().unwrap();
        drop(check);

        cache_update(&manager);
        let download = manager.start_update_download().unwrap();
        let session = download.session();
        assert!(manager.start_check().is_err());
        assert!(manager.start_update_download().is_err());
        assert!(manager.start_install().is_err());
        assert!(manager.start_developer_install().is_err());
        manager.cancel_download().unwrap();
        assert!(session.is_cancelled());
        drop(download);

        let developer_install = manager.start_developer_install().unwrap();
        assert!(manager.start_check().is_err());
        assert!(manager.start_update_download().is_err());
        assert!(manager.start_install().is_err());
        assert!(manager.start_developer_install().is_err());
        assert!(manager.cancel_download().is_err());
        drop(developer_install);
        assert_eq!(manager.status().operation, UpdateOperationStatus::Idle);
    }

    #[test]
    fn failed_or_cancelled_download_preserves_available_without_pending() {
        let manager = UpdateManager::default();
        cache_update(&manager);

        let download = manager.start_update_download().unwrap();
        manager.cancel_download().unwrap();
        drop(download);

        let status = manager.status();
        assert!(status.available_update.is_some());
        assert!(status.pending_update.is_none());
        manager.start_update_download().unwrap();
    }

    #[test]
    fn completed_download_registers_pending_and_blocks_check_and_download() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        complete_download(&manager);

        let status = manager.status();
        assert_eq!(status.operation, UpdateOperationStatus::Idle);
        assert_eq!(
            status.available_update.as_ref().unwrap().version_name,
            "1.3.0"
        );
        assert_eq!(
            status.pending_update.as_ref().unwrap().version_name,
            "1.3.0"
        );
        assert!(manager.start_check().is_err());
        assert!(manager.start_update_download().is_err());
        assert!(manager.start_developer_install().is_err());
    }

    #[test]
    fn published_download_wins_over_a_late_cancellation_request() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        let download = manager.start_update_download().unwrap();

        manager.cancel_download().unwrap();
        download.complete(PathBuf::from("C:/OEA/cache/downloads/update.zip"));

        let status = manager.status();
        assert_eq!(status.operation, UpdateOperationStatus::Idle);
        assert!(status.pending_update.is_some());
    }

    #[test]
    fn install_atomically_consumes_pending_and_failure_does_not_restore_it() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        complete_download(&manager);

        let install = manager.start_install().unwrap();
        assert_eq!(
            install.package_path(),
            Path::new("C:/OEA/cache/downloads/update.zip")
        );
        let status = manager.status();
        assert_eq!(status.operation, UpdateOperationStatus::Installing);
        assert!(status.pending_update.is_none());
        assert!(manager.start_install().is_err());
        drop(install);

        let status = manager.status();
        assert_eq!(status.operation, UpdateOperationStatus::Idle);
        assert!(status.pending_update.is_none());
        assert!(status.available_update.is_some());
        manager.start_update_download().unwrap();
    }

    #[test]
    fn successful_install_keeps_installing_until_process_exit() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        complete_download(&manager);

        manager.start_install().unwrap().complete();

        assert_eq!(
            manager.status().operation,
            UpdateOperationStatus::Installing
        );
        assert!(manager.is_installing());
    }

    #[test]
    fn developer_install_never_creates_pending_update() {
        let manager = UpdateManager::default();

        manager.start_developer_install().unwrap().complete();

        let status = manager.status();
        assert_eq!(status.operation, UpdateOperationStatus::Installing);
        assert!(status.pending_update.is_none());
        assert!(status.available_update.is_none());
    }

    #[tokio::test]
    async fn cancellation_actively_wakes_download_waiters() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        let download = manager.start_update_download().unwrap();
        let session = download.session();
        let cancellation = session.cancellation();

        manager.cancel_download().unwrap();

        tokio::time::timeout(
            std::time::Duration::from_millis(50),
            cancellation.cancelled(),
        )
        .await
        .expect("取消等待者应被立即唤醒");
        assert!(session.is_cancelled());
    }

    #[test]
    fn stale_download_completion_does_not_clear_active_operation() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        let download = manager.start_update_download().unwrap();
        let active_id = download.id();

        manager.finish_download(active_id.wrapping_add(1));

        assert!(manager.start_check().is_err());
        drop(download);
        manager.start_check().unwrap();
    }

    #[test]
    fn status_serialization_hides_pending_package_path() {
        let manager = UpdateManager::default();
        cache_update(&manager);
        complete_download(&manager);

        let json = serde_json::to_value(manager.status()).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "operation": "idle",
                "availableUpdate": { "versionName": "1.3.0", "releaseNote": "notes" },
                "pendingUpdate": { "versionName": "1.3.0", "releaseNote": "notes" }
            })
        );
        assert!(!json.to_string().contains("update.zip"));
    }
}
