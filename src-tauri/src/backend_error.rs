//! 对外错误只携带失败事实。原始诊断由命令边界记录到日志。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "operation", content = "reason", rename_all = "camelCase")]
#[ts(export, export_to = "errors/")]
pub enum BackendError {
    Screenshot(ScreenshotError),
    ArchiveScan(ArchiveScanError),
    UpdateCheck(UpdateError),
    UpdateDownload(UpdateError),
    UpdateInstall(UpdateError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/")]
pub enum ScreenshotError {
    GameEnvironment { reason: GameEnvironmentError },
    CaptureFailed,
    EncodingFailed,
}

/// 扫描与截图共用的可行动游戏环境事实，可作为 `anyhow` 错误链的类型化来源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/")]
pub enum GameEnvironmentError {
    WindowUnavailable,
    HdrEnabled,
    UnsupportedResolution {
        width: u32,
        height: u32,
    },
    WindowSizeChanged {
        expected_width: u32,
        expected_height: u32,
        actual_width: u32,
        actual_height: u32,
    },
}

impl std::fmt::Display for GameEnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowUnavailable => write!(f, "未找到游戏窗口，请先打开游戏"),
            Self::HdrEnabled => write!(f, "游戏所在显示器已开启 HDR，请关闭后重试"),
            Self::UnsupportedResolution { width, height } => {
                write!(f, "游戏分辨率 {width}×{height} 不支持，期待 16:9 分辨率")
            }
            Self::WindowSizeChanged {
                expected_width,
                expected_height,
                actual_width,
                actual_height,
            } => write!(
                f,
                "游戏窗口尺寸由 {expected_width}×{expected_height} 变为 {actual_width}×{actual_height}"
            ),
        }
    }
}
impl std::error::Error for GameEnvironmentError {}

/// 扫描失败只保存稳定原因，底层诊断留在工作线程日志中。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/")]
pub enum ArchiveScanError {
    GameEnvironment { reason: GameEnvironmentError },
    ThreadStartFailed,
    CaptureFailed,
    NavigationFailed,
    ExecutionFailed,
}
impl std::fmt::Display for ArchiveScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "档案扫描失败: {self:?}")
    }
}
impl std::error::Error for ArchiveScanError {}

/// 更新来源提供可行动原因，底层诊断只写日志。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/")]
pub enum UpdateError {
    Busy,
    NoUpdate,
    ProxyConfiguration,
    Network,
    InvalidMetadata,
    Service {
        #[ts(type = "number")]
        code: i64,
    },
    VersionMismatch {
        expected: String,
        actual: String,
    },
    PackageUnavailable {
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
