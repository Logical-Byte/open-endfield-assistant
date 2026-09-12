use std::path::{Path, PathBuf};

/// 在临时下载文件发布到最终路径前管理其生命周期。
pub(super) struct DownloadTarget {
    destination_path: PathBuf,
    staging_path: Option<PathBuf>,
}

impl DownloadTarget {
    pub(super) fn new(destination_path: PathBuf, session_id: u64) -> Result<Self, String> {
        if let Some(parent) = destination_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("无法创建下载目录: {error}"))?;
        }

        let staging_path = PathBuf::from(format!(
            "{}.{}.downloading",
            destination_path.display(),
            session_id
        ));
        Ok(Self {
            destination_path,
            staging_path: Some(staging_path),
        })
    }

    pub(super) fn staging_path(&self) -> &Path {
        self.staging_path
            .as_deref()
            .expect("下载文件发布后不能再访问临时路径")
    }

    pub(super) fn publish(mut self) -> Result<PathBuf, String> {
        let staging_path = self.staging_path();
        std::fs::rename(staging_path, &self.destination_path)
            .map_err(|error| format!("重命名临时文件失败: {error}"))?;
        self.staging_path = None;
        Ok(self.destination_path.clone())
    }
}

impl Drop for DownloadTarget {
    fn drop(&mut self) {
        if let Some(path) = self.staging_path.take() {
            // 同步删除可避免旧任务的延迟清理误删新任务创建的同名临时文件。
            let _ = std::fs::remove_file(path);
        }
    }
}
