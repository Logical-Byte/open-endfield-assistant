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

/// 精确匹配窗口条件，指定的条件必须同时满足。未指定的字段不参与匹配。
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowQuery<'a> {
    pub class_name: Option<&'a str>,
    pub title: Option<&'a str>,
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
            let rect = windows::window::get_client_rect(*self)?;
            anyhow::ensure!(rect.width() > 0 && rect.height() > 0, "窗口客户区尺寸无效");
            Ok(ClientSize {
                width: rect.width() as u32,
                height: rect.height() as u32,
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
