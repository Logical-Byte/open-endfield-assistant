use std::{
    thread,
    time::{Duration, Instant},
};

use ::windows::Win32::{
    Foundation::RECT,
    UI::{
        HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
        Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_MENU,
            VK_RETURN,
        },
        WindowsAndMessaging::{
            GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetForegroundWindow, GetMenu, GetWindowLongPtrW,
            GetWindowRect, IsIconic, IsWindow, IsZoomed, SW_RESTORE, SWP_ASYNCWINDOWPOS,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, ShowWindowAsync,
            WINDOW_EX_STYLE, WINDOW_STYLE, WS_CAPTION, WS_POPUP,
        },
    },
};
use anyhow::{Context, bail, ensure};
use tracing::debug;

use crate::platform::WindowHandle;

use super::window_operation::ensure_foreground_and_topmost;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const TRANSITION_TIMEOUT: Duration = Duration::from_secs(5);
const SIZE_SETTLE_TIME: Duration = Duration::from_millis(300);

/// 通过游戏的 `Alt+Enter` 切换退出全屏，再调整实际客户区像素尺寸。
pub(in crate::platform) fn ensure_windowed_client_size(
    window: WindowHandle,
    width: i32,
    height: i32,
) -> anyhow::Result<()> {
    let hwnd = window.raw;
    ensure!(width > 0 && height > 0, "窗口客户区尺寸必须为正数");
    ensure!(
        !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "游戏窗口无效"
    );

    if unsafe { IsIconic(hwnd) }.as_bool() || unsafe { IsZoomed(hwnd) }.as_bool() {
        unsafe { ShowWindowAsync(hwnd, SW_RESTORE) }.ok()?;
        wait_until(|| {
            Ok(!unsafe { IsIconic(hwnd) }.as_bool() && !unsafe { IsZoomed(hwnd) }.as_bool())
        })
        .context("恢复游戏窗口超时")?;
    }

    if !is_windowed(window) {
        let activation = ensure_foreground_and_topmost(window);
        // 跨输入队列激活异步完成，以实际前台窗口为准，不能仅依赖请求的返回值。
        wait_until(|| Ok(unsafe { GetForegroundWindow() } == hwnd)).with_context(|| {
            format!("等待游戏窗口获得前台焦点超时，激活请求结果：{activation:?}")
        })?;
        debug!("游戏窗口已获得前台焦点，发送 Alt+Enter 退出全屏");
        send_fullscreen_toggle()?;
        wait_for_windowed_transition(window).context("游戏未能完成退出全屏模式")?;
        debug!("游戏已完成窗口化尺寸恢复，开始调整客户区尺寸");
    }

    let mut client = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client) }?;
    if client.right - client.left != width || client.bottom - client.top != height {
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
    wait_until(|| {
        unsafe { GetClientRect(hwnd, &mut client) }?;
        if is_windowed(window)
            && client.right - client.left == width
            && client.bottom - client.top == height
        {
            Ok(stable_since.get_or_insert_with(Instant::now).elapsed() >= SIZE_SETTLE_TIME)
        } else {
            stable_since = None;
            Ok(false)
        }
    })
    .context("游戏窗口未能稳定到指定的窗口化尺寸")
}

fn wait_for_windowed_transition(window: WindowHandle) -> anyhow::Result<()> {
    let mut previous = None;
    let mut stable_since = Instant::now();
    wait_until(|| {
        let current = windowed_client_size(window)?;
        if current != previous {
            debug!(?current, "游戏窗口化过渡中的客户区尺寸");
            previous = current;
            stable_since = Instant::now();
        }
        Ok(current.is_some() && stable_since.elapsed() >= SIZE_SETTLE_TIME)
    })
}

fn windowed_client_size(window: WindowHandle) -> anyhow::Result<Option<(i32, i32)>> {
    let hwnd = window.raw;
    if !is_windowed(window)
        || unsafe { IsIconic(hwnd) }.as_bool()
        || unsafe { IsZoomed(hwnd) }.as_bool()
    {
        return Ok(None);
    }
    let mut client = RECT::default();
    let mut outer = RECT::default();
    unsafe { GetClientRect(hwnd, &mut client) }?;
    unsafe { GetWindowRect(hwnd, &mut outer) }?;
    let width = client.right - client.left;
    let height = client.bottom - client.top;
    if width <= 0 || height <= 0 {
        return Ok(None);
    }
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32);
    let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32);
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    ensure!(dpi != 0, "无法读取游戏窗口 DPI");
    let has_menu = !unsafe { GetMenu(hwnd) }.is_invalid();
    let mut expected = client;
    unsafe { AdjustWindowRectExForDpi(&mut expected, style, has_menu, ex_style, dpi) }?;
    // Unity 先改 `WS_*`，再恢复窗口几何。样式已改变但外框仍等于全屏客户区时，
    // 引擎的窗口化事务尚未完成，此时发出的 `SetWindowPos` 会被后续恢复覆盖。
    if outer.right - outer.left != expected.right - expected.left
        || outer.bottom - outer.top != expected.bottom - expected.top
    {
        return Ok(None);
    }
    Ok(Some((width, height)))
}

fn is_windowed(window: WindowHandle) -> bool {
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

fn wait_until(mut ready: impl FnMut() -> anyhow::Result<bool>) -> anyhow::Result<()> {
    let started = Instant::now();
    loop {
        if ready()? {
            return Ok(());
        }
        ensure!(started.elapsed() < TRANSITION_TIMEOUT, "等待窗口调整超时");
        thread::sleep(POLL_INTERVAL);
    }
}
