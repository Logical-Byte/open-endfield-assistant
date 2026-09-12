use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use anyhow::{Result, bail};

use super::check::AvailableUpdateMetadata;

/// 集中管理更新下载和安装的后端状态。
pub struct UpdateManager {
    inner: Mutex<UpdateState>,
}

struct UpdateState {
    next_session_id: u64,
    available_update: Option<AvailableUpdateMetadata>,
    operation: UpdateOperation,
}

enum UpdateOperation {
    Idle,
    Checking,
    Downloading(Arc<DownloadSession>),
    Installing,
}

pub(super) struct CheckLease<'a> {
    manager: &'a UpdateManager,
    completed: bool,
}

pub(super) struct DownloadSession {
    id: u64,
    cancelled: AtomicBool,
    downloaded_bytes: AtomicU64,
}

pub(super) struct DownloadLease<'a> {
    manager: &'a UpdateManager,
    session: Arc<DownloadSession>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self {
            inner: Mutex::new(UpdateState {
                next_session_id: 0,
                available_update: None,
                operation: UpdateOperation::Idle,
            }),
        }
    }
}

impl UpdateManager {
    pub(super) fn start_check(&self) -> Result<CheckLease<'_>> {
        let mut state = self.lock_state();
        if !matches!(state.operation, UpdateOperation::Idle) {
            bail!("已有更新操作正在进行，请稍后重试");
        }
        state.available_update = None;
        state.operation = UpdateOperation::Checking;
        Ok(CheckLease {
            manager: self,
            completed: false,
        })
    }

    #[cfg(test)]
    fn available_update(&self) -> Option<AvailableUpdateMetadata> {
        self.lock_state().available_update.clone()
    }

    pub(super) fn start_download(&self) -> Result<DownloadLease<'_>> {
        let mut state = self.lock_state();
        match state.operation {
            UpdateOperation::Idle => {}
            UpdateOperation::Checking => bail!("检查更新期间无法开始下载"),
            UpdateOperation::Downloading(_) => bail!("更新下载已在进行"),
            UpdateOperation::Installing => bail!("安装更新期间无法开始下载"),
        }

        state.next_session_id += 1;
        let session = Arc::new(DownloadSession {
            id: state.next_session_id,
            cancelled: AtomicBool::new(false),
            downloaded_bytes: AtomicU64::new(0),
        });
        state.operation = UpdateOperation::Downloading(Arc::clone(&session));

        Ok(DownloadLease {
            manager: self,
            session,
        })
    }

    pub(super) fn cancel_download(&self) -> Result<()> {
        let state = self.lock_state();
        match &state.operation {
            UpdateOperation::Downloading(session) => {
                session.cancel();
                Ok(())
            }
            UpdateOperation::Idle => bail!("当前没有正在进行的下载"),
            UpdateOperation::Checking => bail!("检查更新期间无法取消下载"),
            UpdateOperation::Installing => bail!("安装更新期间无法取消下载"),
        }
    }

    pub(super) fn begin_install(&self) -> Result<()> {
        let mut state = self.lock_state();
        match state.operation {
            UpdateOperation::Idle => {
                state.operation = UpdateOperation::Installing;
                Ok(())
            }
            UpdateOperation::Checking => bail!("检查更新期间无法开始安装更新"),
            UpdateOperation::Downloading(_) => bail!("下载期间无法开始安装更新"),
            UpdateOperation::Installing => bail!("更新安装已在进行"),
        }
    }

    pub(super) fn finish_install(&self) -> Result<()> {
        let mut state = self.lock_state();
        if !matches!(state.operation, UpdateOperation::Installing) {
            bail!("当前没有正在进行的更新安装");
        }
        state.operation = UpdateOperation::Idle;
        Ok(())
    }

    pub(crate) fn is_installing(&self) -> bool {
        let state = self.lock_state();
        matches!(state.operation, UpdateOperation::Installing)
    }

    fn finish_check(&self, available_update: Option<AvailableUpdateMetadata>) {
        let mut state = self.lock_state();
        if matches!(state.operation, UpdateOperation::Checking) {
            state.available_update = available_update;
            state.operation = UpdateOperation::Idle;
        }
    }

    fn finish_download(&self, session_id: u64) {
        let mut state = self.lock_state();
        if matches!(&state.operation, UpdateOperation::Downloading(session) if session.id == session_id)
        {
            state.operation = UpdateOperation::Idle;
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, UpdateState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
}

impl DownloadSession {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
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
        self.manager.finish_download(self.session.id);
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateManager;
    use crate::update::check::AvailableUpdateMetadata;

    fn metadata(version: &str) -> AvailableUpdateMetadata {
        AvailableUpdateMetadata {
            version_name: version.to_string(),
            release_note: "notes".to_string(),
            mirrorchyan_package: None,
        }
    }

    #[test]
    fn check_replaces_cache_and_all_exit_paths_restore_idle() {
        let manager = UpdateManager::default();
        manager
            .start_check()
            .unwrap()
            .complete(Some(metadata("1.3.0")));
        assert_eq!(manager.available_update(), Some(metadata("1.3.0")));

        let failed_check = manager.start_check().unwrap();
        assert_eq!(manager.available_update(), None);
        assert!(manager.start_download().is_err());
        assert!(manager.begin_install().is_err());
        drop(failed_check);

        let download = manager.start_download().unwrap();
        drop(download);
        manager.begin_install().unwrap();
        manager.finish_install().unwrap();

        manager.start_check().unwrap().complete(None);
        assert_eq!(manager.available_update(), None);
        manager.start_download().unwrap();
    }

    #[test]
    fn check_download_and_install_are_mutually_exclusive() {
        let manager = UpdateManager::default();

        assert!(manager.cancel_download().is_err());

        let check = manager.start_check().unwrap();
        assert!(manager.start_check().is_err());
        assert!(manager.start_download().is_err());
        assert!(manager.begin_install().is_err());
        drop(check);

        let download = manager.start_download().unwrap();
        let session = download.session();
        session.set_downloaded_bytes(10);
        assert_eq!(session.downloaded_bytes(), 10);
        assert!(!session.is_cancelled());
        assert!(manager.start_check().is_err());
        assert!(manager.start_download().is_err());
        assert!(manager.begin_install().is_err());
        manager.cancel_download().unwrap();
        assert!(session.is_cancelled());
        drop(download);

        manager.begin_install().unwrap();
        assert!(manager.begin_install().is_err());
        assert!(manager.start_check().is_err());
        assert!(manager.start_download().is_err());
        assert!(manager.cancel_download().is_err());
        manager.finish_install().unwrap();
        assert!(manager.finish_install().is_err());
    }
}
