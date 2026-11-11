//! Unity 的 Alt+Enter 窗口化协议与窗口几何恢复判定。

use super::{SIZE_SETTLE_TIME, activation, geometry, wait_until};
use crate::Window;
use crate::windows::input;
use ::windows::Win32::{
    Foundation::RECT,
    UI::{
        HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
        Input::KeyboardAndMouse::{VK_MENU, VK_RETURN},
        WindowsAndMessaging::{
            GWL_EXSTYLE, GWL_STYLE, GetMenu, GetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE,
            WS_CAPTION, WS_POPUP,
        },
    },
};
use anyhow::{Context, ensure};
use std::time::Instant;
use tracing::debug;

/// 恢复最小化或最大化窗口，并通过游戏的 `Alt+Enter` 切换退出全屏。
pub(crate) fn ensure_windowed(window: Window) -> anyhow::Result<()> {
    activation::restore(window).context("恢复游戏窗口超时")?;

    if !is_windowed(window) {
        activation::activate_and_wait(window)?;
        debug!("游戏窗口已获得前台焦点，发送 Alt+Enter 退出全屏");
        send_fullscreen_toggle()?;
        wait_for_windowed_transition(window).context("游戏未能完成退出全屏模式")?;
        debug!("游戏已完成窗口化尺寸恢复");
    }

    Ok(())
}

fn wait_for_windowed_transition(window: Window) -> anyhow::Result<()> {
    let mut previous = None;
    let mut stable_since = Instant::now();
    wait_until(|| {
        let current = settled_windowed_client_size(window)?;
        if current != previous {
            debug!(?current, "游戏窗口化过渡中的客户区尺寸");
            previous = current;
            stable_since = Instant::now();
        }
        Ok(current.is_some() && stable_since.elapsed() >= SIZE_SETTLE_TIME)
    })
}

fn settled_windowed_client_size(window: Window) -> anyhow::Result<Option<(i32, i32)>> {
    if !is_windowed(window) || activation::is_minimized(window) || activation::is_maximized(window)
    {
        return Ok(None);
    }
    // Unity 先改 `WS_*`，再恢复窗口几何。样式已改变但窗口仍在过渡时客户区为空，
    // 此时读不到客户区，按 “过渡中” 处理。
    let Some(frame) = geometry::try_client_geometry(window)? else {
        return Ok(None);
    };
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_EXSTYLE) } as u32);
    let dpi = unsafe { GetDpiForWindow(window.raw) };
    ensure!(dpi != 0, "无法读取游戏窗口 DPI");
    let has_menu = !unsafe { GetMenu(window.raw) }.is_invalid();
    let mut expected = RECT {
        right: frame.client_size.0,
        bottom: frame.client_size.1,
        ..Default::default()
    };
    unsafe { AdjustWindowRectExForDpi(&mut expected, style, has_menu, ex_style, dpi) }?;
    // 样式已改变但外框仍等于全屏客户区时，引擎的窗口化事务尚未完成，此时发出的
    // `SetWindowPos` 会被后续恢复覆盖。
    if frame.outer_size()
        != (
            expected.right - expected.left,
            expected.bottom - expected.top,
        )
    {
        return Ok(None);
    }
    Ok(Some(frame.client_size))
}

fn is_windowed(window: Window) -> bool {
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    !style.contains(WS_POPUP) && style.contains(WS_CAPTION)
}

fn send_fullscreen_toggle() -> anyhow::Result<()> {
    input::send_key_sequence(&[
        (VK_MENU.0, false),
        (VK_RETURN.0, false),
        (VK_RETURN.0, true),
        (VK_MENU.0, true),
    ])
}
