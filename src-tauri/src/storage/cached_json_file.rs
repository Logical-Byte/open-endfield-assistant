use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

use crate::platform;

/// 一个由 JSON 文本文件持久化的内存模型。
///
/// 写入通过独立互斥锁串行提交。提交期间不会持有缓存写锁，因此读取者可以继续并行
/// 取得上一份已提交的内存快照。文件替换成功后，缓存才会切换到新模型。
pub(crate) struct CachedJsonFile<T> {
    path: PathBuf,
    current: RwLock<Arc<T>>,
    writer: Mutex<()>,
}

impl<T> CachedJsonFile<T>
where
    T: Serialize + DeserializeOwned,
{
    /// 读取并反序列化现有文件。
    pub(crate) fn at(path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取 JSON 文件失败 [{}]", path.display()))?;
        let initial = serde_json::from_str(&text)
            .with_context(|| format!("反序列化 JSON 文件失败 [{}]", path.display()))?;
        Ok(Self::new(path, initial))
    }

    /// 使用上层提供的初始模型创建缓存，不主动写入文件。
    pub(crate) fn new(path: PathBuf, initial: T) -> Self {
        Self {
            path,
            current: RwLock::new(Arc::new(initial)),
            writer: Mutex::new(()),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// 取得当前模型的共享快照；返回后不再持有缓存锁。
    pub(crate) fn snapshot(&self) -> Arc<T> {
        Arc::clone(
            &self
                .current
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    /// 将新模型序列化并提交到文件，成功后发布新的内存快照。
    pub(crate) fn replace(&self, next: T) -> Result<()> {
        let _writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut text = serde_json::to_string_pretty(&next).context("序列化 JSON 文件失败")?;
        text.push('\n');

        write_text_atomically(&self.path, text.as_bytes())
            .with_context(|| format!("写入 JSON 文件失败 [{}]", self.path.display()))?;

        let mut current = self
            .current
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *current = Arc::new(next);
        Ok(())
    }
}

static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

/// 在目标文件所在目录完整写入临时文件，再原子替换目标文件。
fn write_text_atomically(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "目标路径没有文件名"))?;
    let temporary_id = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{temporary_id}",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        platform::file::replace(&temporary, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestModel {
        value: String,
    }

    #[test]
    fn saves_and_loads_json_model() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.json");
        let cached = CachedJsonFile::new(
            path.clone(),
            TestModel {
                value: "old".to_string(),
            },
        );

        cached
            .replace(TestModel {
                value: "new".to_string(),
            })
            .unwrap();

        assert_eq!(cached.snapshot().value, "new");
        assert_eq!(
            CachedJsonFile::<TestModel>::at(path)
                .unwrap()
                .snapshot()
                .value,
            "new"
        );
    }

    #[test]
    fn publishes_cache_only_after_file_commit() {
        let root = tempfile::tempdir().unwrap();
        let parent_file = root.path().join("not-a-directory");
        fs::write(&parent_file, "occupied").unwrap();
        let cached = CachedJsonFile::new(
            parent_file.join("state.json"),
            TestModel {
                value: "old".to_string(),
            },
        );

        let error = cached
            .replace(TestModel {
                value: "new".to_string(),
            })
            .unwrap_err();

        assert!(error.to_string().contains("写入 JSON 文件失败"));
        assert_eq!(cached.snapshot().value, "old");
    }
}
