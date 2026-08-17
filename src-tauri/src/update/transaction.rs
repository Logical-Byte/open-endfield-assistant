//! 单个更新事务的持久化格式与约定文件名。
//!
//! `transaction.json` 是下载续传、崩溃后重试和 Bootstrap 交接的共同事实来源。
//! 普通应用启动不读取它；正式版资源仍只由可执行文件内置版本号选择。

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// 事务目录中的固定文件名，避免 JSON 提供任意目标路径。
pub const TRANSACTION_FILE: &str = "transaction.json";
/// 尚未完成或尚未校验的下载文件。
pub const PARTIAL_ARTIFACT: &str = "artifact.zip.part";
/// SHA-256 已通过的完整归档。
pub const ARTIFACT: &str = "artifact.zip";
/// 当前旧版本复制出的 Bootstrap 程序。
pub const BOOTSTRAP_EXE: &str = "bootstrap.exe";
/// 解压后的新版本候选入口。
pub const CANDIDATE_EXE: &str = "candidate/OEA.exe";

/// 更新事务已经完成到哪个可恢复阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStage {
    /// 正在写入部分归档，可能可以续传。
    Downloading,
    /// 下载结束，但尚未验证 SHA-256。
    Downloaded,
    /// 归档已校验并发布为 `artifact.zip`。
    Verified,
    /// 候选程序与版本资源已准备完成。
    Prepared,
    /// Bootstrap 副本与调用方 PID 已就绪，可以退出主程序。
    BootstrapReady,
}

/// 事务所使用的实际下载来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSource {
    /// MirrorChyan 完整包。
    Mirrorchyan,
    /// GitHub Release 完整包。
    Github,
}

/// 能证明远端对象是否变化的 HTTP 校验标识（validator）。
///
/// 用枚举保留 header 种类，避免用 `"etag:..."` 这类字符串约定再解析。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum HttpValidator {
    /// HTTP `ETag` header 原值。
    Etag(String),
    /// HTTP `Last-Modified` header 原值。
    LastModified(String),
}

/// `transaction.json` 的完整结构。
///
/// 字段只保存身份、校验和恢复所需信息；最终文件路径均由固定目录布局推导。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// JSON schema 版本，供未来显式迁移或拒绝不兼容格式。
    pub schema_version: u32,
    /// 本次更新的诊断标识。
    pub transaction_id: String,
    /// 当前可恢复阶段。
    pub stage: TransactionStage,
    /// 发起更新的旧程序版本。
    pub source_version: String,
    /// 正在准备的新程序版本。
    pub target_version: String,
    /// 完整包实际来源。
    pub download_source: DownloadSource,
    /// 完整包 URL；重试时必须与已保存事务一致。
    pub artifact_url: String,
    /// 来源提供的完整包 SHA-256。
    pub expected_sha256: String,
    /// 来源或 HTTP 响应提供的预期总大小。
    pub expected_size: Option<u64>,
    /// 支持安全续传的服务端校验标识。
    pub http_validator: Option<HttpValidator>,
    /// Bootstrap 必须等待退出的旧 OEA 进程 ID。
    pub caller_pid: Option<u32>,
}

impl Transaction {
    /// 从固定事务文件读取并反序列化状态。
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("读取事务 {} 失败", path.display()))?;
        let transaction = serde_json::from_slice(&bytes).context("解析更新事务失败")?;
        Ok(transaction)
    }

    /// 把调用时的完整状态写回 JSON。
    ///
    /// 本方法不自动跟踪字段变化；更新工作流修改阶段或续传字段后必须显式调用。
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
    use super::{DownloadSource, HttpValidator, Transaction, TransactionStage};

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
            http_validator: Some(HttpValidator::Etag("etag-value".into())),
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
