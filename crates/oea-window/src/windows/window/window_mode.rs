use std::time::Instant;

use anyhow::{Context, bail, ensure};
use tracing::debug;
use windows::Win32::{
    Foundation::RECT,
    UI::{
        HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
        Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_MENU,
            VK_RETURN,
        },
        WindowsAndMessaging::{
            GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetMenu, GetWindowLongPtrW,
            SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
            WINDOW_EX_STYLE, WINDOW_STYLE, WS_CAPTION, WS_POPUP,
        },
    },
};

use crate::Window;

use super::state::{self, SIZE_SETTLE_TIME};
use super::validate;
use super::window_operation::ensure_foreground_and_topmost;

/// 恢复最小化或最大化窗口，并通过游戏的 `Alt+Enter` 切换退出全屏。
pub(crate) fn ensure_windowed(window: Window) -> anyhow::Result<()> {
    let hwnd = window.raw;
    validate(window)?;
    state::restore(window).context("恢复游戏窗口超时")?;

    if !is_windowed(window) {
        let activation = ensure_foreground_and_topmost(window);
        // 跨输入队列激活异步完成，以实际前台窗口为准，不能仅依赖请求的返回值。
        state::wait_until(|| Ok(unsafe { GetForegroundWindow() } == hwnd)).with_context(|| {
            format!("等待游戏窗口获得前台焦点超时，激活请求结果：{activation:?}")
        })?;
        debug!("游戏窗口已获得前台焦点，发送 Alt+Enter 退出全屏");
        send_fullscreen_toggle()?;
        wait_for_windowed_transition(window).context("游戏未能完成退出全屏模式")?;
        debug!("游戏已完成窗口化尺寸恢复");
    }

    Ok(())
}

/// 将客户区调整为指定物理像素尺寸，并等待尺寸连续稳定。
pub(crate) fn resize_windowed_client(
    window: Window,
    width: i32,
    height: i32,
) -> anyhow::Result<()> {
    let hwnd = window.raw;
    ensure!(width > 0 && height > 0, "窗口客户区尺寸必须为正数");
    validate(window)?;

    let (client_width, client_height) = state::client_size(window)?;
    if client_width != width || client_height != height {
        let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32);
        let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32);
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        ensure!(dpi != 0, "无法读取游戏窗口 DPI");
        let mut outer = RECT {
            right: width,
            bottom: height,
            ..Default::default()
        };
        let has_menu = !unsafe { GetMenu(hwnd) }.is_invalid();
        unsafe { AdjustWindowRectExForDpi(&mut outer, style, has_menu, ex_style, dpi) }?;
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                outer.right - outer.left,
                outer.bottom - outer.top,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
            )
        }?;
    }

    // 窗口消息处理可能分多帧完成，连续稳定后才能创建截图器和坐标转换器。
    let mut stable_since = None;
    state::wait_until(|| {
        let client = state::client_size(window).ok();
        if client == Some((width, height)) {
            Ok(stable_since.get_or_insert_with(Instant::now).elapsed() >= SIZE_SETTLE_TIME)
        } else {
            stable_since = None;
            Ok(false)
        }
    })
    .context("游戏窗口未能稳定到指定的客户区尺寸")
}

fn wait_for_windowed_transition(window: Window) -> anyhow::Result<()> {
    let mut previous = None;
    let mut stable_since = Instant::now();
    state::wait_until(|| {
        let current = windowed_client_size(window)?;
        if current != previous {
            debug!(?current, "游戏窗口化过渡中的客户区尺寸");
            previous = current;
            stable_since = Instant::now();
        }
        Ok(current.is_some() && stable_since.elapsed() >= SIZE_SETTLE_TIME)
    })
}

fn windowed_client_size(window: Window) -> anyhow::Result<Option<(i32, i32)>> {
    if !is_windowed(window) || state::is_iconic(window) || state::is_zoomed(window) {
        return Ok(None);
    }
    // Unity 先改 `WS_*`，再恢复窗口几何。样式已改变但窗口仍在过渡时客户区为空，
    // 此时读不到客户区，按 “过渡中” 处理。
    let Some(frame) = state::try_client_frame(window)? else {
        return Ok(None);
    };
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_EXSTYLE) } as u32);
    let dpi = unsafe { GetDpiForWindow(window.raw) };
    ensure!(dpi != 0, "无法读取游戏窗口 DPI");
    let has_menu = !unsafe { GetMenu(window.raw) }.is_invalid();
    let mut expected = RECT {
        right: frame.width,
        bottom: frame.height,
        ..Default::default()
    };
    unsafe { AdjustWindowRectExForDpi(&mut expected, style, has_menu, ex_style, dpi) }?;
    // 样式已改变但外框仍等于全屏客户区时，引擎的窗口化事务尚未完成，此时发出的
    // `SetWindowPos` 会被后续恢复覆盖。
    if frame.left + frame.right != expected.right - expected.left
        || frame.top + frame.bottom != expected.bottom - expected.top
    {
        return Ok(None);
    }
    Ok(Some((frame.width, frame.height)))
}

fn is_windowed(window: Window) -> bool {
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(window.raw, GWL_STYLE) } as u32);
    !style.contains(WS_POPUP) && style.contains(WS_CAPTION)
}

fn send_fullscreen_toggle() -> anyhow::Result<()> {
    let inputs = [
        (VK_MENU, false),
        (VK_RETURN, false),
        (VK_RETURN, true),
        (VK_MENU, true),
    ]
    .map(|(key, release)| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if release {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    });
    let size = std::mem::size_of::<INPUT>() as i32;
    let written = unsafe { SendInput(&inputs, size) };
    if written != inputs.len() as u32 {
        // 部分写入时也释放两个按键，避免留下按住的 Alt 或 Enter。
        let _ = unsafe { SendInput(&inputs[2..], size) };
        bail!("无法发送退出全屏快捷键，请确认有操作游戏窗口的权限");
    }
    Ok(())
}
