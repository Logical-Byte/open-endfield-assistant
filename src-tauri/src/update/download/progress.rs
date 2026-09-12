use std::{sync::Arc, time::Duration};

use crate::update::{commands::DownloadProgress, manager::DownloadSession};

/// 管理进度采样任务，并保证成功事件一定是该会话的最后一个进度事件。
pub(super) struct ProgressReporter {
    channel: tauri::ipc::Channel<DownloadProgress>,
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl ProgressReporter {
    pub(super) fn start_channel(
        channel: tauri::ipc::Channel<DownloadProgress>,
        session: Arc<DownloadSession>,
        total: u64,
    ) -> Self {
        let channel_for_task = channel.clone();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let mut last_downloaded = 0u64;
            let mut last_instant = tokio::time::Instant::now();
            let mut smoothed_speed = 0.0f64;
            const EMA_ALPHA: f64 = 0.3;

            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        let downloaded = session.downloaded_bytes();
                        let now = tokio::time::Instant::now();
                        let elapsed = now.duration_since(last_instant);
                        if elapsed.is_zero() {
                            continue;
                        }
                        let bytes_in_interval = downloaded.saturating_sub(last_downloaded);
                        let instant_speed = bytes_in_interval as f64 / elapsed.as_secs_f64();
                        smoothed_speed = if smoothed_speed == 0.0 {
                            instant_speed
                        } else {
                            EMA_ALPHA * instant_speed + (1.0 - EMA_ALPHA) * smoothed_speed
                        };
                        let progress = if total > 0 {
                            ((downloaded as f64 / total as f64) * 100.0).min(100.0)
                        } else {
                            0.0
                        };
                        let _ = channel_for_task.send(DownloadProgress {
                            downloaded_size: downloaded,
                            total_size: total,
                            speed: smoothed_speed as u64,
                            progress,
                        });
                        last_downloaded = downloaded;
                        last_instant = now;
                    }
                }
            }
        });

        Self {
            channel,
            stop_tx: Some(stop_tx),
            task: Some(task),
        }
    }

    pub(super) async fn stop(&mut self) {
        self.signal_stop();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }

    pub(super) fn emit_complete(&self, downloaded: u64, total: u64) {
        let _ = self.channel.send(DownloadProgress {
            downloaded_size: downloaded,
            total_size: if total > 0 { total } else { downloaded },
            speed: 0,
            progress: 100.0,
        });
    }

    fn signal_stop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.signal_stop();
    }
}
