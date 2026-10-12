//! 游戏窗口连接、显示条件检查与识别坐标转换。

mod error;
mod resolution;

pub use error::Error;
pub(crate) use resolution::{Resolution, ResolutionTransform};

use tracing::{debug, warn};

use crate::platform::{self, WindowHandle};

pub(crate) struct Requirements {
    pub restore_if_minimized: bool,
    pub ensure_on_screen: bool,
    pub hdr_disabled: bool,
}

impl Default for Requirements {
    fn default() -> Self {
        Self {
            restore_if_minimized: true,
            ensure_on_screen: true,
            hdr_disabled: true,
        }
    }
}

pub(crate) struct Environment {
    pub window: WindowHandle,
    pub resolution: Resolution,
}

/// 按要求准备窗口并检查显示条件。所有调用都检查客户区尺寸有效性。
pub(crate) fn connect(requirements: Requirements) -> Result<Environment, Error> {
    let window = platform::window::get_window_by_title(
        Some(platform::window::ENDFIELD_WINDOW_CLASS),
        Some(platform::window::ENDFIELD_WINDOW_TITLE),
    )
    .map_err(|error| {
        debug!(error = ?error, "定位游戏窗口失败");
        Error::WindowUnavailable
    })?;
    if requirements.restore_if_minimized {
        let _ = platform::window::restore_window_if_minimized(window)
            .inspect_err(|error| warn!("恢复窗口失败: {error:#}"));
    }
    if requirements.ensure_on_screen {
        let _ = platform::window::ensure_window_on_screen(window)
            .inspect_err(|error| warn!("确保窗口在屏幕上失败: {error:#}"));
    }

    let rect = platform::window::get_client_rect(window).map_err(|error| {
        debug!(error = ?error, "读取游戏窗口尺寸失败");
        Error::WindowUnavailable
    })?;
    let resolution = client_resolution(rect.width(), rect.height())?;

    if requirements.hdr_disabled {
        match platform::window::hdr::is_hdr_enabled_on_window_monitor(window) {
            Ok(true) => return Err(Error::HdrEnabled),
            Ok(false) => {}
            Err(error) => warn!("检查显示器 HDR 状态失败: {error:#}，继续执行任务"),
        }
    }

    Ok(Environment { window, resolution })
}

fn client_resolution(width: i32, height: i32) -> Result<Resolution, Error> {
    let (Ok(width), Ok(height)) = (u32::try_from(width), u32::try_from(height)) else {
        debug!(width, height, "游戏窗口尺寸无效");
        return Err(Error::WindowUnavailable);
    };
    Resolution::new(width, height)
}
