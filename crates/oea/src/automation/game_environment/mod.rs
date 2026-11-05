//! 游戏窗口连接、显示条件检查与识别坐标转换。

mod error;
mod resolution;

pub use error::Error;
pub use resolution::{Resolution, ResolutionTransform};

use tracing::{debug, warn};

use oea_window::{Window, WindowQuery};

pub struct Requirements {
    pub restore_if_minimized: bool,
    pub ensure_on_screen: bool,
    pub hdr_disabled: bool,
    pub windowed_720p: bool,
}

impl Default for Requirements {
    fn default() -> Self {
        Self {
            restore_if_minimized: true,
            ensure_on_screen: true,
            hdr_disabled: true,
            windowed_720p: true,
        }
    }
}

pub struct Environment {
    pub window: Window,
    pub resolution: Resolution,
}

/// 按要求准备窗口并检查显示条件。所有调用都检查客户区尺寸有效性。
pub fn connect(requirements: Requirements) -> Result<Environment, Error> {
    let window = oea_window::search(window_query())
        .map_err(|error| {
            debug!(error = ?error, "定位游戏窗口失败");
            Error::WindowUnavailable
        })?
        .ok_or(Error::WindowUnavailable)?;
    if requirements.restore_if_minimized && !requirements.windowed_720p {
        let _ = window
            .restore_if_minimized()
            .inspect_err(|error| warn!("恢复窗口失败: {error:#}"));
    }
    if requirements.windowed_720p {
        window
            .ensure_windowed()
            .and_then(|()| window.resize_windowed_client(1280, 720))
            .map_err(|error| {
                warn!(error = %error, "自动调整游戏窗口为 1280×720 窗口模式失败");
                debug!(error = ?error, "游戏窗口调整失败详情");
                Error::WindowCannotResizeTo720p
            })?;
    }
    if requirements.ensure_on_screen {
        let _ = window
            .ensure_on_screen(requirements.windowed_720p)
            .inspect_err(|error| warn!("确保窗口在屏幕上失败: {error:#}"));
    }

    let size = window.client_size().map_err(|error| {
        debug!(error = ?error, "读取游戏窗口尺寸失败");
        Error::WindowUnavailable
    })?;
    let resolution = Resolution::new(size.width, size.height)?;
    if requirements.windowed_720p && (resolution.width != 1280 || resolution.height != 720) {
        warn!(%resolution, "游戏窗口在定位后未保持 1280×720");
        return Err(Error::WindowCannotResizeTo720p);
    }

    if requirements.hdr_disabled {
        match window.is_hdr_enabled() {
            Ok(true) => return Err(Error::HdrEnabled),
            Ok(false) => {}
            Err(error) => warn!("检查显示器 HDR 状态失败: {error:#}，继续执行任务"),
        }
    }

    Ok(Environment { window, resolution })
}

/// 游戏窗口选择条件集中于应用侧，供连接流程和热键前台规则复用。
pub(crate) fn window_query() -> WindowQuery<'static> {
    WindowQuery {
        class_name: Some("UnityWndClass"),
        title: Some("Endfield"),
    }
}
