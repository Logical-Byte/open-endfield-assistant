//! 更新错误只携带领域原因，诊断在底层失败转换时记录。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 更新操作的结构化失败原因。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "errors/", rename = "UpdateError")]
pub struct Error {
    pub reason: Reason,
}

impl Error {
    pub fn new(reason: Reason) -> Self {
        Self { reason }
    }

    pub fn failed(reason: Reason, source: anyhow::Error) -> Self {
        tracing::debug!(reason = ?reason, error = ?source, "更新操作失败诊断");
        Self::new(reason)
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::failed(Reason::Failed, anyhow::Error::msg(message))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "更新操作结束: {:?}", self.reason)
    }
}

impl std::error::Error for Error {}

/// 更新流程的类型化原因。由 `kind` 区分：
/// - `cancelled`：用户主动取消操作。
/// - `busy`：已有互斥更新操作正在运行。
/// - `noUpdate`：没有可下载或安装的更新。
/// - `proxyConfiguration`：代理配置无法使用。
/// - `network`：更新网络请求失败。
/// - `invalidMetadata`：更新元数据缺失或格式无效。
/// - `service`：更新服务拒绝请求，`code` 为服务返回的原始错误码。
/// - `versionMismatch`：下载目标与服务返回的版本不一致，`expected` 为请求版本，`actual` 为返回版本。
/// - `packageUnavailable`：目标版本没有可用安装包，`version` 为该版本。
/// - `integrity`：更新包完整性检查失败。
/// - `fileAccess`：更新文件读写失败。
/// - `debugBuild`：开发构建不支持此更新操作。
/// - `invalidPackage`：更新包结构或内容无效。
/// - `preparation`：更新事务准备失败。
/// - `helperStart`：无法启动更新辅助进程。
/// - `failed`：未进一步归类的更新失败。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "UpdateErrorReason")]
pub enum Reason {
    Cancelled,
    Busy,
    NoUpdate,
    ProxyConfiguration,
    Network,
    InvalidMetadata,
    Service {
        /// 更新服务返回的原始错误码。
        #[ts(type = "number")]
        code: i64,
    },
    VersionMismatch {
        /// 请求下载的版本。
        expected: String,
        /// 服务返回的版本。
        actual: String,
    },
    PackageUnavailable {
        /// 缺少下载包的版本。
        version: String,
    },
    Integrity,
    FileAccess,
    DebugBuild,
    InvalidPackage,
    Preparation,
    HelperStart,
    Failed,
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };

    use super::Reason;
    use crate::update;

    struct LogWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for LogWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn logs_diagnostics_before_returning_the_public_reason() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let output = Arc::clone(&logs);
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(move || LogWriter(Arc::clone(&output)))
            .finish();
        let source = anyhow::Error::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        ))
        .context("写入更新包失败");
        let error = tracing::subscriber::with_default(subscriber, || {
            update::Error::failed(Reason::FileAccess, source)
        });
        let log = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
        assert!(log.contains("写入更新包失败"));
        assert!(log.contains("access denied"));
        let serialized = serde_json::to_value(&error).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!({ "reason": { "kind": "fileAccess" } })
        );
    }
}
