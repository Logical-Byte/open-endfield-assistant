use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const TRANSACTION_FILE: &str = "transaction.json";
pub const PARTIAL_ARTIFACT: &str = "artifact.zip.part";
pub const ARTIFACT: &str = "artifact.zip";
pub const BOOTSTRAP_EXE: &str = "bootstrap.exe";
pub const CANDIDATE_EXE: &str = "candidate/OEA.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStage {
    Downloading,
    Downloaded,
    Verified,
    Prepared,
    BootstrapReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSource {
    Mirrorchyan,
    Github,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub schema_version: u32,
    pub transaction_id: String,
    pub stage: TransactionStage,
    pub source_version: String,
    pub target_version: String,
    pub download_source: DownloadSource,
    pub artifact_url: String,
    pub expected_sha256: String,
    pub expected_size: Option<u64>,
    pub http_validator: Option<String>,
    pub caller_pid: Option<u32>,
}

impl Transaction {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("读取事务 {} 失败", path.display()))?;
        let transaction = serde_json::from_slice(&bytes).context("解析更新事务失败")?;
        Ok(transaction)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("事务文件缺少父目录")?;
        fs::create_dir_all(parent).context("创建事务目录失败")?;
        let bytes = serde_json::to_vec_pretty(self).context("序列化更新事务失败")?;
        fs::write(path, bytes).context("写入更新事务失败")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadSource, Transaction, TransactionStage};

    #[test]
    fn transaction_round_trips_all_resume_fields() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("transaction.json");
        let transaction = Transaction {
            schema_version: 1,
            transaction_id: "tx-1".into(),
            stage: TransactionStage::Downloading,
            source_version: "0.1.0".into(),
            target_version: "0.2.0".into(),
            download_source: DownloadSource::Github,
            artifact_url: "https://example.test/OEA.zip".into(),
            expected_sha256: "a".repeat(64),
            expected_size: Some(42),
            http_validator: Some("etag-value".into()),
            caller_pid: Some(1234),
        };

        transaction.save(&path).unwrap();

        assert_eq!(Transaction::load(&path).unwrap(), transaction);

        let mut updated = transaction.clone();
        updated.stage = TransactionStage::Verified;
        updated.save(&path).unwrap();
        assert_eq!(Transaction::load(&path).unwrap(), updated);
    }
}
