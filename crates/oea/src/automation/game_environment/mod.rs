//! 游戏窗口连接、显示条件检查与识别坐标转换。

mod error;
mod resolution;

pub use error::Error;
pub use resolution::{Resolution, ResolutionTransform};

use tracing::{debug, warn};

use oea_window::{ContentKind, TitleMatch, Window, WindowQuery};

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

/// 支持的终末地运行方式。两种窗口同时存在时按声明顺序优先。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Windows 客户端，客户区就是游戏画面。
    Desktop,
    /// 浏览器中的云游戏，游戏画面是客户区内的网页视口。
    Cloud,
}

impl Target {
    const ALL: [Self; 2] = [Self::Desktop, Self::Cloud];

    fn window_query(self) -> WindowQuery<'static> {
        match self {
            Self::Desktop => WindowQuery {
                class_name: Some("UnityWndClass"),
                title: Some(TitleMatch::Exact("Endfield")),
            },
            // 浏览器会在标题后追加站点名，只能按包含匹配。
            Self::Cloud => WindowQuery {
                class_name: Some("Chrome_WidgetWin_1"),
                title: Some(TitleMatch::Contains("云·终末地")),
            },
        }
    }

    fn content_kind(self) -> ContentKind {
        match self {
            Self::Desktop => ContentKind::Client,
            Self::Cloud => ContentKind::BrowserViewport,
        }
    }

    /// 把游戏画面调整为 1280×720。
    ///
    /// 客户端通过 `Alt+Enter` 退出全屏后调整客户区；云游戏没有可用的全屏快捷键，
    /// 按浏览器实测视口边距反推客户区，让网页视口等于 1280×720。
    fn prepare(self, window: Window) -> anyhow::Result<()> {
        match self {
            Self::Desktop => window
                .ensure_windowed()
                .and_then(|()| window.resize_windowed_client(1280, 720)),
            Self::Cloud => window.ensure_viewport_content_size(1280, 720),
        }
    }
}

pub struct Environment {
    pub window: Window,
    pub resolution: Resolution,
    /// 720p 识别坐标与窗口客户区物理坐标之间的转换。
    pub transform: ResolutionTransform,
}

/// 按要求准备窗口并检查显示条件。所有调用都检查客户区尺寸有效性。
pub fn connect(requirements: Requirements) -> Result<Environment, Error> {
    let (target, window) = find_window().ok_or(Error::WindowUnavailable)?;
    debug!(?target, "已定位终末地窗口");

    if requirements.restore_if_minimized && !requirements.windowed_720p {
        let _ = window
            .restore_if_minimized()
            .inspect_err(|error| warn!("恢复窗口失败: {error:#}"));
    }
    if requirements.windowed_720p {
        target.prepare(window).map_err(|error| {
            warn!(error = %error, "自动将终末地画面调整为 1280×720 失败");
            debug!(error = ?error, "游戏窗口调整失败详情");
            Error::WindowCannotResizeTo720p
        })?;
    }
    if requirements.ensure_on_screen {
        let _ = window
            .ensure_on_screen(requirements.windowed_720p)
            .inspect_err(|error| warn!("确保窗口在屏幕上失败: {error:#}"));
    }

    let content = window
        .content_rect(target.content_kind())
        .map_err(|error| {
            debug!(error = ?error, "读取游戏画面区域失败");
            Error::WindowUnavailable
        })?;
    let size = window.client_size().map_err(|error| {
        debug!(error = ?error, "读取游戏窗口尺寸失败");
        Error::WindowUnavailable
    })?;
    let client = Resolution::new(size.width, size.height)?;
    let resolution = Resolution::new(content.width, content.height)?;
    if requirements.windowed_720p && (resolution.width != 1280 || resolution.height != 720) {
        warn!(%resolution, "游戏画面在定位后未保持 1280×720");
        return Err(Error::WindowCannotResizeTo720p);
    }

    // 云游戏的画面由浏览器合成，桌面为 SDR 内容，不受显示器 HDR 输出影响。
    if requirements.hdr_disabled && target == Target::Desktop {
        match window.is_hdr_enabled() {
            Ok(true) => return Err(Error::HdrEnabled),
            Ok(false) => {}
            Err(error) => warn!("检查显示器 HDR 状态失败: {error:#}，继续执行任务"),
        }
    }

    let transform = ResolutionTransform::new(client, content)?;

    Ok(Environment {
        window,
        resolution,
        transform,
    })
}

/// 任一终末地窗口位于前台。供任务热键判断适用范围。
pub(crate) fn is_game_window_foreground() -> bool {
    Target::ALL.into_iter().any(|target| {
        window_at(target.window_query())
            .and_then(|window| window.is_foreground().ok())
            .unwrap_or(false)
    })
}

fn find_window() -> Option<(Target, Window)> {
    Target::ALL
        .into_iter()
        .find_map(|target| window_at(target.window_query()).map(|window| (target, window)))
}

fn window_at(query: WindowQuery<'static>) -> Option<Window> {
    match oea_window::search(query) {
        Ok(window) => window,
        Err(error) => {
            debug!(error = ?error, "定位游戏窗口失败");
            None
        }
    }
}
