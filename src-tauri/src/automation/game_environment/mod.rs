//! 游戏窗口连接、显示条件检查与识别坐标转换。

mod error;
mod resolution;

pub use error::Error;
pub(crate) use resolution::{Resolution, ResolutionTransform};

use tracing::{debug, info, warn};

use crate::platform::{self, WindowHandle};

pub(crate) struct Requirements {
    pub hdr_disabled: bool,
}

impl Default for Requirements {
    fn default() -> Self {
        Self { hdr_disabled: true }
    }
}

pub(crate) struct Environment {
    pub window: WindowHandle,
    pub resolution: Resolution,
}

/// 连接时恢复最小化窗口并确保它在屏幕上。
pub(crate) fn connect(requirements: Requirements) -> Result<Environment, Error> {
    let window = platform::window::get_window_by_title(
        Some(platform::window::ENDFIELD_WINDOW_CLASS),
        Some(platform::window::ENDFIELD_WINDOW_TITLE),
    )
    .map_err(|error| {
        debug!(error = ?error, "定位游戏窗口失败");
        Error::WindowUnavailable
    })?;
    let _ = platform::window::restore_window_if_minimized(window)
        .inspect_err(|error| warn!("恢复窗口失败: {error:#}"));
    let _ = platform::window::ensure_window_on_screen(window)
        .inspect_err(|error| warn!("确保窗口在屏幕上失败: {error:#}"));

    let rect = platform::window::get_client_rect(window).map_err(|error| {
        debug!(error = ?error, "读取游戏窗口尺寸失败");
        Error::WindowUnavailable
    })?;
    let width = u32::try_from(rect.width()).map_err(|error| {
        debug!(width = rect.width(), error = ?error, "游戏窗口宽度无效");
        Error::WindowUnavailable
    })?;
    let height = u32::try_from(rect.height()).map_err(|error| {
        debug!(height = rect.height(), error = ?error, "游戏窗口高度无效");
        Error::WindowUnavailable
    })?;
    let resolution = Resolution::new(width, height)?;
    info!("游戏分辨率: {}×{}", resolution.width(), resolution.height());

    if requirements.hdr_disabled {
        match platform::window::hdr::is_hdr_enabled_on_window_monitor(window) {
            Ok(true) => return Err(Error::HdrEnabled),
            Ok(false) => {}
            Err(error) => warn!("检查显示器 HDR 状态失败: {error:#}，继续执行任务"),
        }
    }

    Ok(Environment { window, resolution })
}
