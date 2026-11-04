//! 浏览器窗口的内容区域（网页视口）判定与尺寸调整。
//!
//! 浏览器窗口的客户区包含自绘的标签栏与地址栏，游戏画面只占其中的网页视口。
//! 网页视口由 UI Automation 的 `Document` 元素矩形给出，`GetClientRect` 无法区分两者。

use std::time::Instant;

use ::windows::Win32::Foundation::{HWND, POINT, RECT};
use ::windows::Win32::Graphics::Gdi::ClientToScreen;
use ::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use ::windows::Win32::System::Variant::{VARIANT, VT_I4};
use ::windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId,
    UIA_DocumentControlTypeId,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetWindowLongPtrW, WINDOW_STYLE, WS_POPUP,
};
use anyhow::{Context, ensure};

use crate::{ContentKind, ContentRect, Window};

use super::state::{self, SIZE_SETTLE_TIME};

pub(crate) fn content_rect(window: Window, kind: ContentKind) -> anyhow::Result<ContentRect> {
    let (width, height) = state::client_size(window)?;
    match kind {
        ContentKind::Client => Ok(ContentRect {
            x: 0,
            y: 0,
            width: width as u32,
            height: height as u32,
        }),
        ContentKind::BrowserViewport => {
            let top = viewport_top(window)?;
            Ok(ContentRect {
                x: 0,
                y: top,
                width: width as u32,
                height: (height - top) as u32,
            })
        }
    }
}

/// 调整窗口，使网页视口的物理像素尺寸等于指定尺寸。
///
/// 客户区比视口多出浏览器工具栏的高度，因此每次调整前重新测量视口偏移，
/// 按实测偏移反推目标客户区尺寸，直到客户区与视口同时稳定。
pub(crate) fn ensure_viewport_content_size(
    window: Window,
    width: i32,
    height: i32,
) -> anyhow::Result<()> {
    ensure!(width > 0 && height > 0, "窗口视口尺寸必须为正数");
    state::restore(window).context("恢复云游戏窗口超时")?;

    // 全屏窗口的视口跟随显示器，无法通过调整窗口尺寸控制。
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    ensure!(
        !style.contains(WS_POPUP),
        "云游戏窗口处于全屏状态，请先退出全屏"
    );

    let mut stable_since = None;
    state::wait_until(|| {
        let top = viewport_top(window)?;
        let (client_width, client_height) = state::client_size(window)?;
        if client_width == width && client_height == height + top {
            return Ok(stable_since.get_or_insert_with(Instant::now).elapsed() >= SIZE_SETTLE_TIME);
        }
        stable_since = None;
        state::set_client_size(window, width, height + top)?;
        Ok(false)
    })
    .context("云游戏窗口未能稳定到指定视口尺寸")
}

/// 读取网页视口上沿在客户区中的物理像素偏移。
fn viewport_top(window: Window) -> anyhow::Result<i32> {
    let (_, client_height) = state::client_size(window)?;
    let viewport = document_viewport(window.raw)?;
    let mut origin = POINT::default();
    unsafe { ClientToScreen(window.raw, &mut origin) }
        .ok()
        .context("转换客户区坐标失败")?;
    let top = viewport.top - origin.y;
    ensure!((0..client_height).contains(&top), "网页视口位置超出客户区");
    Ok(top)
}

/// 通过 UI Automation 读取浏览器当前标签页的网页视口矩形（物理像素）。
fn document_viewport(hwnd: HWND) -> anyhow::Result<RECT> {
    let _apartment = ComApartment::enter();
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
            .context("创建 UI Automation 实例失败")?;
    let root = unsafe { automation.ElementFromHandle(hwnd) }
        .context("读取窗口的 UI Automation 元素失败")?;

    let mut value = VARIANT::default();
    // 控件类型属性按 `VT_I4` 传递，值是 `UIA_DocumentControlTypeId`。
    unsafe {
        let inner = &mut *value.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = UIA_DocumentControlTypeId.0;
    }
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
