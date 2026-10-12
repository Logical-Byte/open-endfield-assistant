//! 游戏环境失败事实，不依赖具体自动化任务。
use serde::Serialize;
use ts_rs::TS;

/// 自动化与截图共用的可行动游戏环境原因。
///
/// 由 `kind` 区分原因：
/// - `windowUnavailable`：未能取得游戏窗口或窗口尺寸。
/// - `hdrEnabled`：游戏显示器开启 HDR，截图颜色无法保证。
/// - `unsupportedResolution`：游戏窗口分辨率不支持，`width` / `height` 为观察到的像素尺寸。
/// - `windowSizeChanged`：当前截图尺寸与会话开始时不同，
///   `expected_width` / `expected_height` 为原始尺寸，`actual_width` / `actual_height` 为当前尺寸。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "GameEnvironmentError")]
pub enum Error {
    WindowUnavailable,
    HdrEnabled,
    UnsupportedResolution {
        /// 观察到的游戏窗口宽度，单位像素。
        width: u32,
        /// 观察到的游戏窗口高度，单位像素。
        height: u32,
    },
    WindowSizeChanged {
        /// 会话开始时的窗口宽度，单位像素。
        expected_width: u32,
        /// 会话开始时的窗口高度，单位像素。
        expected_height: u32,
        /// 当前截图宽度，单位像素。
        actual_width: u32,
        /// 当前截图高度，单位像素。
        actual_height: u32,
    },
}

impl std::fmt::Display for Error {
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
impl std::error::Error for Error {}
