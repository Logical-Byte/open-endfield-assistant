//! 目标窗口查找、原始客户区截图与物理输入。
//!
//! 坐标和尺寸均使用客户区物理像素。窗口引用不拥有系统窗口，不自动重新查找。
//! 物理输入会操作系统光标和前台窗口，调用方应串行使用输入器。

mod input;
#[cfg(windows)]
mod windows;

pub use input::{Input, InputPolicy, InputPolicyBuilder, MouseButton, ScrollDelta};

use image::RgbaImage;

/// 客户区内的物理像素坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// 一次查询观察到的客户区物理像素尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSize {
    pub width: u32,
    pub height: u32,
}

/// 窗口内容区域在客户区中的位置与尺寸，使用物理像素。
///
/// 浏览器窗口的客户区还包含自绘的标签栏与地址栏，游戏画面只占据其中的网页视口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 内容区域的判定方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    /// 整个客户区，例如原生游戏客户端。
    Client,
    /// 浏览器网页视口，排除标签栏、地址栏等浏览器自身界面。
    BrowserViewport,
}

/// 窗口标题匹配方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleMatch<'a> {
    /// 与窗口标题完全一致。
    Exact(&'a str),
    /// 窗口标题包含指定文本。浏览器会改写标题，云游戏只能按包含匹配。
    Contains(&'a str),
}

/// 精确匹配窗口条件，指定的条件必须同时满足。未指定的字段不参与匹配。
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowQuery<'a> {
    pub class_name: Option<&'a str>,
    pub title: Option<TitleMatch<'a>>,
}

/// 查找第一个匹配的顶层窗口，未找到返回 `None`。
///
/// 不恢复窗口、不改变前台窗口。非 Windows 平台返回 unsupported error。
pub fn search(query: WindowQuery<'_>) -> anyhow::Result<Option<Window>> {
    #[cfg(windows)]
    {
        windows::window::search(query)
    }
    #[cfg(not(windows))]
    {
        let _ = query;
        Err(unsupported())
    }
}

/// 非拥有型窗口引用。窗口关闭后操作会失败，系统也可能重用原生窗口标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    #[cfg(windows)]
    raw: ::windows::Win32::Foundation::HWND,
}

// 仅保存非拥有型 HWND，不持有线程关联资源。输入屏蔽只存在于同步方法调用内部。
#[cfg(windows)]
unsafe impl Send for Window {}

impl Window {
    /// 读取当前客户区尺寸，不缓存。零尺寸或窗口已失效时返回错误。
    pub fn client_size(&self) -> anyhow::Result<ClientSize> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            let (width, height) = windows::window::client_size(*self)?;
            Ok(ClientSize {
                width: width as u32,
                height: height as u32,
            })
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 捕获完整客户区，返回物理尺寸 RGBA 像素。默认使用 `PrintWindow`。
    ///
    /// 不缩放、不编码、不激活窗口。截图尺寸可能与此前查询不同。调用为同步操作，
    /// 会等待目标窗口处理截图请求，不保证最小化或特殊渲染窗口能够提供有效画面。
    pub fn screenshot(&self) -> anyhow::Result<RgbaImage> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::capture::PrintWindowState::new(*self).screencap()
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 查询目标窗口是否位于前台，不改变前台窗口。
    pub fn is_foreground(&self) -> anyhow::Result<bool> {
        #[cfg(windows)]
        {
            windows::validate(*self)?;
            Ok(windows::window::is_foreground(*self))
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 尝试激活目标窗口并移到 Z 序顶部，沿用现有前台激活逻辑。
    pub fn activate(&self) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            windows::validate(*self)?;
            windows::window::ensure_foreground_and_topmost(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 窗口最小化时将其恢复。
    pub fn restore_if_minimized(&self) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            windows::window::restore_window_if_minimized(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 恢复最小化或最大化窗口并退出全屏，等待游戏完成窗口化。
    pub fn ensure_windowed(&self) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::ensure_windowed(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 将客户区调整为指定物理像素尺寸，并等待尺寸连续稳定。
    pub fn resize_windowed_client(&self, width: i32, height: i32) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::resize_windowed_client(*self, width, height)
        }
        #[cfg(not(windows))]
        {
            let _ = (width, height);
            Err(unsupported())
        }
    }

    /// 读取内容区域在客户区中的矩形，不缓存。
    ///
    /// `ContentKind::BrowserViewport` 通过 UI Automation 读取网页视口，读不到时返回错误。
    pub fn content_rect(&self, kind: ContentKind) -> anyhow::Result<ContentRect> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::content_rect(*self, kind)
        }
        #[cfg(not(windows))]
        {
            let _ = kind;
            Err(unsupported())
        }
    }

    /// 调整窗口，使网页视口的物理像素尺寸等于指定尺寸并保持稳定。
    ///
    /// 浏览器窗口的客户区包含自绘的工具栏，外框尺寸由实测的视口偏移反推，
    /// 不能使用 `AdjustWindowRectExForDpi` 预测。窗口全屏或读不到网页视口时返回错误。
    pub fn ensure_viewport_content_size(&self, width: i32, height: i32) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::ensure_viewport_content_size(*self, width, height)
        }
        #[cfg(not(windows))]
        {
            let _ = (width, height);
            Err(unsupported())
        }
    }

    /// 查询窗口是否处于全屏显示，不改变窗口状态。
    ///
    /// 同时识别 `WS_POPUP` 全屏与浏览器 `F11` 全屏（无标题栏且外框铺满显示器）。
    pub fn is_fullscreen(&self) -> anyhow::Result<bool> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::is_fullscreen(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 让全屏窗口退出全屏。非全屏窗口直接返回成功。
    ///
    /// 云游戏页面会锁住键盘，只有浏览器自身的「长按 Esc」手势能退出全屏。
    pub fn exit_fullscreen(&self) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::validate(*self)?;
            windows::window::exit_fullscreen(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }

    /// 将客户区移回显示器工作区，可选择保留尺寸。跳过最小化和最大化窗口。
    pub fn ensure_on_screen(&self, preserve_size: bool) -> anyhow::Result<()> {
        #[cfg(windows)]
        {
            let _dpi = windows::DpiContext::enter()?;
            windows::window::ensure_window_on_screen(*self, preserve_size)
        }
        #[cfg(not(windows))]
        {
            let _ = preserve_size;
            Err(unsupported())
        }
    }

    /// 查询窗口所在显示器是否启用 HDR。查询失败由调用方决定是否继续。
    pub fn is_hdr_enabled(&self) -> anyhow::Result<bool> {
        #[cfg(windows)]
        {
            windows::validate(*self)?;
            windows::window::hdr::is_hdr_enabled_on_window_monitor(*self)
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }
}

#[cfg(not(windows))]
fn unsupported() -> anyhow::Error {
    anyhow::anyhow!(
        "window operations are not supported on {}",
        std::env::consts::OS
    )
}
