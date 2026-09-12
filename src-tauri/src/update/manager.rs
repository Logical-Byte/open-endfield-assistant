use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use anyhow::{Result, bail};

/// 集中管理更新下载和安装的后端状态。
pub struct UpdateManager {
    inner: Mutex<UpdateState>,
}

struct UpdateState {
    next_session_id: u64,
    phase: UpdatePhase,
}

enum UpdatePhase {
    Idle,
    Downloading(Arc<DownloadSession>),
    Installing,
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
                phase: UpdatePhase::Idle,
            }),
        }
    }
}

impl UpdateManager {
    pub(super) fn start_download(&self) -> Result<DownloadLease<'_>> {
        let mut state = self.lock_state();
        if matches!(state.phase, UpdatePhase::Installing) {
            bail!("安装更新期间无法开始下载");
        }

        if let UpdatePhase::Downloading(previous) = &state.phase {
            previous.cancel();
        }

        state.next_session_id += 1;
        let session = Arc::new(DownloadSession {
            id: state.next_session_id,
            cancelled: AtomicBool::new(false),
            downloaded_bytes: AtomicU64::new(0),
        });
        state.phase = UpdatePhase::Downloading(Arc::clone(&session));

        Ok(DownloadLease {
            manager: self,
            session,
        })
    }

    pub(super) fn cancel_download(&self) -> Result<()> {
        let state = self.lock_state();
        match &state.phase {
            UpdatePhase::Downloading(session) => {
                session.cancel();
                Ok(())
            }
            UpdatePhase::Idle => bail!("当前没有正在进行的下载"),
            UpdatePhase::Installing => bail!("安装更新期间无法取消下载"),
        }
    }

    pub(super) fn begin_install(&self) -> Result<()> {
        let mut state = self.lock_state();
        match state.phase {
            UpdatePhase::Idle => {
                state.phase = UpdatePhase::Installing;
                Ok(())
            }
            UpdatePhase::Downloading(_) => bail!("下载期间无法开始安装更新"),
            UpdatePhase::Installing => bail!("更新安装已在进行"),
        }
    }

    pub(super) fn finish_install(&self) -> Result<()> {
        let mut state = self.lock_state();
        if !matches!(state.phase, UpdatePhase::Installing) {
            bail!("当前没有正在进行的更新安装");
        }
        state.phase = UpdatePhase::Idle;
        Ok(())
    }

    pub(crate) fn is_installing(&self) -> bool {
        let state = self.lock_state();
        matches!(state.phase, UpdatePhase::Installing)
    }

    fn finish_download(&self, session_id: u64) {
        let mut state = self.lock_state();
        if matches!(&state.phase, UpdatePhase::Downloading(session) if session.id == session_id) {
            state.phase = UpdatePhase::Idle;
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, UpdateState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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

    #[test]
    fn replacing_download_isolates_sessions_and_ignores_stale_completion() {
        let manager = UpdateManager::default();
        let old = manager.start_download().unwrap();
        let old_session = old.session();
        old_session.set_downloaded_bytes(10);

        let new = manager.start_download().unwrap();
        let new_session = new.session();
        new_session.set_downloaded_bytes(20);

        assert!(old_session.is_cancelled());
        assert!(!new_session.is_cancelled());
        assert_eq!(old_session.downloaded_bytes(), 10);
        assert_eq!(new_session.downloaded_bytes(), 20);

        drop(old);
        assert!(manager.begin_install().is_err());

        drop(new);
        manager.begin_install().unwrap();
    }

    #[test]
    fn install_transitions_reject_wrong_phase_operations() {
        let manager = UpdateManager::default();

        assert!(manager.cancel_download().is_err());
        manager.begin_install().unwrap();
        assert!(manager.begin_install().is_err());
        assert!(manager.start_download().is_err());
        assert!(manager.cancel_download().is_err());
        manager.finish_install().unwrap();

        let download = manager.start_download().unwrap();
        assert!(manager.begin_install().is_err());
        drop(download);
        manager.begin_install().unwrap();
        manager.finish_install().unwrap();
        assert!(manager.finish_install().is_err());
    }
}
