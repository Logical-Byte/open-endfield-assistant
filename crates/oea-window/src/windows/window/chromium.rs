//! 浏览器窗口的内容区域（网页视口）判定与尺寸调整。
//!
//! 浏览器窗口的客户区包含自绘的标签栏与地址栏，游戏画面只占其中的网页视口。
//! 网页视口由 UI Automation 的 `Document` 元素矩形给出，`GetClientRect` 无法区分两者。

use std::time::{Duration, Instant};

use ::windows::Win32::Foundation::{HWND, POINT, RECT};
use ::windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MONITORINFO};
use ::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use ::windows::Win32::System::Variant::VARIANT;
use ::windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId,
    UIA_DocumentControlTypeId,
};
use ::windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetWindowLongPtrW, GetWindowRect, WINDOW_STYLE, WS_CAPTION, WS_POPUP,
};
use anyhow::{Context, ensure};
use tracing::debug;

use crate::windows::input;
use crate::{ContentRect, Window};

use super::{SIZE_SETTLE_TIME, activation, geometry, wait_until};

/// 浏览器退出全屏需要保持 Esc 按下的时长。
const FULLSCREEN_EXIT_HOLD: Duration = Duration::from_secs(3);

pub(super) fn viewport_rect(window: Window) -> anyhow::Result<ContentRect> {
    let (width, height) = geometry::client_size(window)?;
    let insets = viewport_insets(window)?;
    Ok(ContentRect {
        x: insets.left,
        y: insets.top,
        width: (width - insets.left - insets.right) as u32,
        height: (height - insets.top - insets.bottom) as u32,
    })
}

/// 调整窗口，使网页视口的物理像素尺寸等于指定尺寸。
///
/// 客户区比视口多出浏览器工具栏等浏览器自身界面，因此每次调整前重新测量视口边距，
/// 按实测边距反推目标客户区尺寸，直到客户区与视口同时稳定。
///
/// 全屏时先激活浏览器并长按 `Esc` 退出，失败则中止调整。
pub(crate) fn resize_viewport(window: Window, width: i32, height: i32) -> anyhow::Result<()> {
    ensure!(width > 0 && height > 0, "窗口视口尺寸必须为正数");
    activation::restore(window).context("恢复云游戏窗口超时")?;
    exit_fullscreen(window)?;

    let mut stable_since = None;
    wait_until(|| {
        let insets = viewport_insets(window)?;
        let (client_width, client_height) = geometry::client_size(window)?;
        let target_width = width + insets.left + insets.right;
        let target_height = height + insets.top + insets.bottom;
        if client_width == target_width && client_height == target_height {
            return Ok(stable_since.get_or_insert_with(Instant::now).elapsed() >= SIZE_SETTLE_TIME);
        }
        stable_since = None;
        geometry::set_client_size(window, target_width, target_height)?;
        Ok(false)
    })
    .context("云游戏窗口未能稳定到指定视口尺寸")
}

/// 让浏览器退出全屏。
///
/// 云游戏页面在播放时会锁住键盘，普通按键（包括 `F11` 与短按 `Esc`）到不了浏览器；
/// 但 Chromium 把「长按 `Esc`」当作退出全屏的手势，键盘锁不会拦截它。
/// 全屏窗口的几何由浏览器锁定，不先退出全屏就无法调整视口尺寸。
fn exit_fullscreen(window: Window) -> anyhow::Result<()> {
    if !is_fullscreen(window)? {
        return Ok(());
    }
    // 物理按键只发送给前台窗口，跨输入队列激活异步完成，以实际前台窗口为准。
    activation::activate_and_wait(window)?;

    debug!("长按 Esc 退出云游戏窗口全屏");
    input::hold_key(VK_ESCAPE.0, FULLSCREEN_EXIT_HOLD)?;
    wait_until(|| Ok(!is_fullscreen(window)?)).context("长按 Esc 后云游戏窗口仍处于全屏")
}

/// 判断窗口是否处于全屏显示。
///
/// 两种形式：设置了 `WS_POPUP` 的独占/无边框全屏，以及 Chromium 的 `F11` 全屏。
/// 后者去掉标题栏与边框，外框正好铺满显示器，但不用 `WS_POPUP`。
fn is_fullscreen(window: Window) -> anyhow::Result<bool> {
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    if style.contains(WS_POPUP) {
        return Ok(true);
    }
    if style.contains(WS_CAPTION) {
        return Ok(false);
    }

    let monitor = geometry::monitor(window)?;
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info) }.ok()?;
    let mut outer = RECT::default();
    unsafe { GetWindowRect(window.raw, &mut outer) }?;
    let monitor_rect = info.rcMonitor;
    Ok(outer.left == monitor_rect.left
        && outer.top == monitor_rect.top
        && outer.right == monitor_rect.right
        && outer.bottom == monitor_rect.bottom)
}

/// 视口相对客户区的边距，已消除舍入误差，且保证能在客户区内切出视口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Insets {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// `document_viewport` 换算到客户区坐标后的矩形。
#[derive(Debug, Clone, Copy)]
struct MeasuredViewport {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

/// UIA 矩形由 DIP 换算到物理像素，存在约 2 px 舍入，也可能略超出客户区。
/// 小于该容差的差值按「紧密贴合」处理，不当作真实留白。
const INSET_TOLERANCE: i32 = 4;

/// 读取视口相对客户区的边距。
fn viewport_insets(window: Window) -> anyhow::Result<Insets> {
    let (client_width, client_height) = geometry::client_size(window)?;
    let insets = insets_for(measured_viewport(window)?, (client_width, client_height));
    ensure!(
        insets.left + insets.right < client_width && insets.top + insets.bottom < client_height,
        "网页视口尺寸无效"
    );
    Ok(insets)
}

fn measured_viewport(window: Window) -> anyhow::Result<MeasuredViewport> {
    let viewport = document_viewport(window.raw)?;
    let mut origin = POINT::default();
    unsafe { ClientToScreen(window.raw, &mut origin) }
        .ok()
        .context("转换客户区坐标失败")?;
    Ok(MeasuredViewport {
        left: viewport.left - origin.x,
        top: viewport.top - origin.y,
        width: viewport.right - viewport.left,
        height: viewport.bottom - viewport.top,
    })
}

/// 把视口矩形换算成客户区内的边距。
///
/// 浏览器工具栏固定在客户区顶部；侧边栏、开发者工具等停靠面板会让视口在左侧或
/// 右侧收窄。先把矩形夹到客户区内，再把不大于 `INSET_TOLERANCE` 的差值归零。
fn insets_for(viewport: MeasuredViewport, client: (i32, i32)) -> Insets {
    let (client_width, client_height) = client;
    let left = viewport.left.clamp(0, client_width);
    let top = viewport.top.clamp(0, client_height);
    let width = viewport.width.clamp(0, client_width - left);
    let height = viewport.height.clamp(0, client_height - top);
    let right = client_width - left - width;
    let bottom = client_height - top - height;

    let (left, right) = if left + right <= INSET_TOLERANCE {
        (0, 0)
    } else {
        (left, right)
    };
    let top = if top <= INSET_TOLERANCE { 0 } else { top };
    let bottom = if bottom <= INSET_TOLERANCE { 0 } else { bottom };

    Insets {
        left,
        top,
        right,
        bottom,
    }
}

/// 通过 UI Automation 读取浏览器当前标签页的网页视口矩形（物理像素）。
fn document_viewport(hwnd: HWND) -> anyhow::Result<RECT> {
    let _apartment = ComApartment::enter();
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
            .context("创建 UI Automation 实例失败")?;
    let root = unsafe { automation.ElementFromHandle(hwnd) }
        .context("读取窗口的 UI Automation 元素失败")?;

    let value = VARIANT::from(UIA_DocumentControlTypeId.0);
    let condition =
        unsafe { automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &value) }
            .context("创建 UI Automation 查询条件失败")?;
    let document = unsafe { root.FindFirst(TreeScope_Descendants, &condition) }
        .context("未找到浏览器网页视口，请确认云游戏页面已打开")?;
    let rect = unsafe { document.CurrentBoundingRectangle() }.context("读取网页视口矩形失败")?;
    ensure!(
        rect.right > rect.left && rect.bottom > rect.top,
        "浏览器网页视口矩形无效"
    );
    Ok(rect)
}

/// 只用当前调用平衡 `CoInitializeEx`：线程已按其他套间模型初始化时也可以继续使用。
struct ComApartment(bool);

impl ComApartment {
    fn enter() -> Self {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self(result.is_ok())
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Insets, MeasuredViewport, insets_for};

    fn viewport(left: i32, top: i32, width: i32, height: i32) -> MeasuredViewport {
        MeasuredViewport {
            left,
            top,
            width,
            height,
        }
    }

    fn insets(left: i32, top: i32, right: i32, bottom: i32) -> Insets {
        Insets {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn rounding_noise_is_treated_as_flush() {
        // UIA 典型输出：左侧多 1 px、宽高各多 1~3 px。
        let measured = viewport(1, 57, 1281, 723);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 57, 0, 0));
    }

    #[test]
    fn toolbar_height_is_kept() {
        let measured = viewport(0, 56, 1280, 720);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 56, 0, 0));
    }

    #[test]
    fn left_docked_panel_is_kept() {
        let measured = viewport(320, 57, 960, 719);
        assert_eq!(insets_for(measured, (1280, 776)), insets(320, 57, 0, 0));
    }

    #[test]
    fn right_docked_panel_is_kept() {
        let measured = viewport(0, 57, 800, 719);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 57, 480, 0));
    }

    #[test]
    fn bottom_bar_is_kept() {
        let measured = viewport(0, 57, 1280, 700);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 57, 0, 19));
    }

    #[test]
    fn fullscreen_viewport_has_no_insets() {
        let measured = viewport(0, 0, 1280, 776);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 0, 0, 0));
    }

    #[test]
    fn viewport_overflowing_client_is_clamped() {
        let measured = viewport(-2, -3, 1400, 900);
        assert_eq!(insets_for(measured, (1280, 776)), insets(0, 0, 0, 0));
    }
}
