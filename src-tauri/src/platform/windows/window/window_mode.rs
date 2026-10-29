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
            IsIconic, IsWindow, IsZoomed, SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, ShowWindowAsync, WINDOW_EX_STYLE, WINDOW_STYLE,
            WS_CAPTION, WS_POPUP,
        },
    },
};
use anyhow::{Context, bail, ensure};

use crate::platform::WindowHandle;

use super::window_operation::ensure_foreground_and_topmost;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const TRANSITION_TIMEOUT: Duration = Duration::from_secs(5);
const SIZE_SETTLE_TIME: Duration = Duration::from_millis(300);

/// 通过游戏的 Alt+Enter 切换退出全屏，再调整实际客户区像素尺寸。
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
        ensure_foreground_and_topmost(window)?;
        ensure!(
            unsafe { GetForegroundWindow() } == hwnd,
            "游戏窗口未获得前台焦点"
        );
        send_fullscreen_toggle()?;
        wait_until(|| Ok(is_windowed(window))).context("游戏未能退出全屏模式")?;
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

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ::windows::{
        Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                CreateWindowExW, DispatchMessageW, GetClientRect, GetMessageW, MSG, PostMessageW,
                WM_CLOSE, WS_EX_NOACTIVATE, WS_MAXIMIZE, WS_MINIMIZE, WS_OVERLAPPEDWINDOW,
            },
        },
        core::w,
    };
    use scopeguard::defer;

    use super::*;

    #[test]
    fn windowed_client_size_is_exact_and_idempotent() {
        super::super::window_operation::set_thread_dpi_awareness_context();
        for initial_state in [WINDOW_STYLE::default(), WS_MINIMIZE, WS_MAXIMIZE] {
            // 独立消息循环模拟游戏线程，验证跨线程异步恢复与缩放。
            let (sender, receiver) = mpsc::channel();
            let owner = thread::spawn(move || {
                super::super::window_operation::set_thread_dpi_awareness_context();
                let hwnd = unsafe {
                    CreateWindowExW(
                        WS_EX_NOACTIVATE,
                        w!("STATIC"),
                        w!("OEA window sizing test"),
                        WS_OVERLAPPEDWINDOW | initial_state,
                        0,
                        0,
                        640,
                        480,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                .unwrap();
                sender.send(hwnd.0 as usize).unwrap();
                let mut message = MSG::default();
                while unsafe { IsWindow(Some(hwnd)) }.as_bool() {
                    if unsafe { GetMessageW(&mut message, None, 0, 0) }.0 <= 0 {
                        break;
                    }
                    unsafe { DispatchMessageW(&message) };
                }
            });
            let hwnd = HWND(receiver.recv().unwrap() as *mut _);
            defer! {
                let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, Default::default(), Default::default()) };
                owner.join().unwrap();
            }
            let window = WindowHandle { raw: hwnd };
            for _ in 0..2 {
                ensure_windowed_client_size(window, 1280, 720).unwrap();
                let mut rect = RECT::default();
                unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
                assert_eq!(
                    (rect.right - rect.left, rect.bottom - rect.top),
                    (1280, 720)
                );
                assert!(is_windowed(window));
                assert!(!unsafe { IsIconic(hwnd) }.as_bool());
                assert!(!unsafe { IsZoomed(hwnd) }.as_bool());
            }
            super::super::window_operation::ensure_window_on_screen(window, true).unwrap();
            let mut rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
            assert_eq!(
                (rect.right - rect.left, rect.bottom - rect.top),
                (1280, 720)
            );
        }
    }

    #[test]
    fn windowed_client_size_rejects_invalid_windows_and_dimensions() {
        let window = WindowHandle {
            raw: HWND::default(),
        };
        assert!(ensure_windowed_client_size(window, 1280, 720).is_err());
        assert!(ensure_windowed_client_size(window, 0, 720).is_err());
    }
}
