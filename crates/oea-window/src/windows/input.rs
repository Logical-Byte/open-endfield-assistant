use ::windows::Win32::Foundation::POINT;
use ::windows::Win32::Graphics::Gdi::ClientToScreen;
use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    BlockInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_WHEEL, MOUSEINPUT, MapVirtualKeyW, SendInput, VIRTUAL_KEY,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SetCursorPos,
};
use anyhow::{Context, ensure};

use crate::{MouseButton, Point, ScrollDelta, Window};

mod buttons;

pub(crate) struct State {
    window: Window,
    last_pos: Option<Point>,
}

impl State {
    pub(crate) fn new(window: Window) -> Self {
        Self {
            window,
            last_pos: None,
        }
    }

    pub(crate) fn prepare(&self, activate: bool) -> anyhow::Result<()> {
        super::validate(self.window)?;
        if activate {
            self.window.activate()?;
        }
        Ok(())
    }

    fn to_screen(&self, point: Point) -> anyhow::Result<POINT> {
        let mut screen = point.into();
        unsafe { ClientToScreen(self.window.raw, &mut screen) }
            .ok()
            .context("转换客户区坐标失败")?;
        Ok(screen)
    }

    pub(crate) fn mouse_down(&mut self, button: MouseButton, point: Point) -> anyhow::Result<()> {
        let screen = self.to_screen(point)?;
        unsafe { SetCursorPos(screen.x, screen.y) }.context("定位鼠标失败")?;
        let flags = button.to_mouse_down_flags();
        send(mouse(MOUSE_EVENT_FLAGS(flags.flags), flags.button_data))?;
        self.last_pos = Some(point);
        Ok(())
    }

    pub(crate) fn move_mouse(&mut self, point: Point) -> anyhow::Result<()> {
        let screen = self.to_screen(point)?;
        // 沿用现有物理输入的主显示器归一化规则。
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        ensure!(width > 0 && height > 0, "屏幕尺寸无效");
        let mut event = mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, 0);
        event.Anonymous.mi.dx = (i64::from(screen.x) * 65535 / i64::from(width)) as i32;
        event.Anonymous.mi.dy = (i64::from(screen.y) * 65535 / i64::from(height)) as i32;
        send(event)?;
        self.last_pos = Some(point);
        Ok(())
    }

    pub(crate) fn mouse_up(&mut self, button: MouseButton) -> anyhow::Result<()> {
        let flags = button.to_mouse_up_flags();
        send(mouse(MOUSE_EVENT_FLAGS(flags.flags), flags.button_data))
    }

    pub(crate) fn scroll(&mut self, delta: ScrollDelta) -> anyhow::Result<()> {
        let target = match self.last_pos {
            Some(point) => point,
            None => {
                let size = self.window.client_size()?;
                Point {
                    x: (size.width / 2) as i32,
                    y: (size.height / 2) as i32,
                }
            }
        };
        let screen = self.to_screen(target)?;
        unsafe { SetCursorPos(screen.x, screen.y) }.context("定位滚动位置失败")?;
        if delta.vertical != 0 {
            send(mouse(MOUSEEVENTF_WHEEL, delta.vertical as u32))?;
        }
        if delta.horizontal != 0 {
            send(mouse(MOUSEEVENTF_HWHEEL, delta.horizontal as u32))?;
        }
        Ok(())
    }

    pub(crate) fn key_down(&mut self, key: u16) -> anyhow::Result<()> {
        send(keyboard(key, false))
    }
    pub(crate) fn key_up(&mut self, key: u16) -> anyhow::Result<()> {
        send(keyboard(key, true))
    }
}

fn mouse(flags: MOUSE_EVENT_FLAGS, data: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                mouseData: data,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn keyboard(key: u16, released: bool) -> INPUT {
    let mut keyboard = KEYBDINPUT {
        wVk: VIRTUAL_KEY(key),
        wScan: unsafe { MapVirtualKeyW(u32::from(key), MAPVK_VK_TO_VSC) } as u16,
        ..Default::default()
    };
    if released {
        keyboard.dwFlags = KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: keyboard },
    }
}

fn send(event: INPUT) -> anyhow::Result<()> {
    let written = unsafe { SendInput(&[event], std::mem::size_of::<INPUT>() as i32) };
    ensure!(
        written == 1,
        "发送物理输入失败：{}",
        ::windows::core::Error::from_thread()
    );
    Ok(())
}

/// 屏蔽只在同步方法内创建和解除，保证始终在同一线程执行。
pub(crate) struct InputBlock {
    active: bool,
}

impl InputBlock {
    pub(crate) fn enter(enabled: bool) -> anyhow::Result<Self> {
        if enabled {
            unsafe { BlockInput(true) }.context("屏蔽真实键鼠输入失败")?;
        }
        Ok(Self { active: enabled })
    }

    pub(crate) fn release(&mut self) -> anyhow::Result<()> {
        if self.active {
            unsafe { BlockInput(false) }.context("解除真实键鼠输入屏蔽失败")?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for InputBlock {
    fn drop(&mut self) {
        if let Err(source) = self.release() {
            tracing::debug!(error = ?source, "解除输入屏蔽失败");
        }
    }
}
