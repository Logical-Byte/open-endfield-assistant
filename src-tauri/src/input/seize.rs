use anyhow::{Result, bail};
use scopeguard::defer;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    BlockInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MAPVK_VK_TO_VSC, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, MapVirtualKeyW, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, HWND_NOTOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOMOVE,
    SWP_NOSIZE, SetCursorPos, SetWindowPos,
};

use crate::input::{base::InputBase, input_utils::Contact};
use crate::utils::point::Point2D;
use crate::window::ensure_foreground_and_topmost;

pub struct SeizeInput {
    hwnd: HWND,
    block_input: bool,
    last_pos: Option<(i32, i32)>,
}

impl Drop for SeizeInput {
    fn drop(&mut self) {
        let _ = self.unblock_input();
    }
}

impl SeizeInput {
    fn new(hwnd: HWND, block_input: bool) -> Self {
        Self {
            hwnd,
            block_input,
            last_pos: None,
        }
    }

    fn touch_down(&mut self, contact: Contact, x: i32, y: i32) -> Result<()> {
        let mut point = POINT { x, y };

        if !self.hwnd.is_invalid() {
            self.ensure_foreground()?;
            unsafe { ClientToScreen(self.hwnd, &mut point) }.ok()?;
        }

        unsafe { SetCursorPos(point.x, point.y) }?;

        let flags = contact.to_mouse_down_flags();

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    mouseData: flags.button_data,
                    dwFlags: flags.flags,
                    ..Default::default()
                },
            },
        };

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for touch_down");
        }

        self.last_pos = Some((x, y));

        Ok(())
    }

    fn touch_move(&mut self, _contact: Contact, x: i32, y: i32) -> Result<()> {
        let mut point = POINT { x, y };

        if !self.hwnd.is_invalid() {
            self.ensure_foreground()?;
            unsafe { ClientToScreen(self.hwnd, &mut point) }.ok()?;
        }

        // 使用 SendInput + MOUSEEVENTF_MOVE + MOUSEEVENTF_ABSOLUTE 移动光标
        // 需要将屏幕坐标转换为 0-65535 范围的归一化坐标
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if screen_width <= 0 || screen_height <= 0 {
            bail!(
                "GetSystemMetrics returned invalid screen size: {}×{}",
                screen_width,
                screen_height,
            );
        }

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: (point.x * 65535) / screen_width,
                    dy: (point.y * 65535) / screen_height,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    ..Default::default()
                },
            },
        };

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for touch_move");
        }

        self.last_pos = Some((x, y));

        Ok(())
    }

    fn touch_up(&self, contact: Contact, _x: i32, _y: i32) -> Result<()> {
        if !self.hwnd.is_invalid() {
            self.ensure_foreground()?;
        }

        defer! {
            let _ = self.unblock_input();
        }

        let flags = contact.to_mouse_up_flags();

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    mouseData: flags.button_data,
                    dwFlags: flags.flags,
                    ..Default::default()
                },
            },
        };

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for touch_up");
        }

        Ok(())
    }

    fn ensure_foreground(&self) -> Result<()> {
        ensure_foreground_and_topmost(self.hwnd)
    }

    fn get_target_pos(&self) -> (i32, i32) {
        if let Some(pos) = self.last_pos {
            return pos;
        }
        // 未设置时返回窗口客户区中心
        if !self.hwnd.is_invalid() {
            let mut rect = RECT::default();
            if unsafe { GetClientRect(self.hwnd, &mut rect) }.is_ok() {
                return ((rect.right - rect.left) / 2, (rect.bottom - rect.top) / 2);
            }
        }
        (0, 0)
    }

    fn check_and_block_input(&self) -> Result<()> {
        if self.block_input {
            unsafe { BlockInput(true) }?;
        }
        Ok(())
    }

    fn unblock_input(&self) -> Result<()> {
        if self.block_input {
            unsafe { BlockInput(false) }?;
        }
        Ok(())
    }

    fn inactive(&self) -> Result<()> {
        self.unblock_input()?;
        if !self.hwnd.is_invalid() {
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    Some(HWND_NOTOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                )
            }?;
        }
        Ok(())
    }

    fn relative_move(&mut self, dx: i32, dy: i32) -> Result<()> {
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        self.ensure_foreground()?;

        self.check_and_block_input()?;
        defer! {
            let _ = self.unblock_input();
        }

        let mut input: INPUT = unsafe { std::mem::zeroed() };
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dx = dx;
        input.Anonymous.mi.dy = dy;
        input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE;

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for relative_move");
        }
        Ok(())
    }

    fn input_text(&self, text: &str) -> Result<()> {
        self.ensure_foreground()?;

        let u16_text: Vec<u16> = text.encode_utf16().collect();

        let mut input_vec: Vec<INPUT> = Vec::with_capacity(u16_text.len() * 2);
        for &ch in &u16_text {
            // KEYDOWN (UNICODE)
            let mut input: INPUT = unsafe { std::mem::zeroed() };
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE;
            input.Anonymous.ki.wScan = ch;
            input_vec.push(input);

            // KEYUP (UNICODE)
            let mut input: INPUT = unsafe { std::mem::zeroed() };
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
            input.Anonymous.ki.wScan = ch;
            input_vec.push(input);
        }

        let written = unsafe { SendInput(&input_vec, std::mem::size_of::<INPUT>() as i32) };
        if written != input_vec.len() as u32 {
            bail!(
                "SendInput wrote {} but expected {}",
                written,
                input_vec.len()
            );
        }
        Ok(())
    }

    fn key_down(&self, key: i32) -> Result<()> {
        self.ensure_foreground()?;

        let mut input: INPUT = unsafe { std::mem::zeroed() };
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki.wVk = VIRTUAL_KEY(key as u16);
        input.Anonymous.ki.wScan = unsafe { MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC) } as u16;

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for key_down");
        }
        Ok(())
    }

    fn key_up(&self, key: i32) -> Result<()> {
        self.ensure_foreground()?;

        let mut input: INPUT = unsafe { std::mem::zeroed() };
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki.wVk = VIRTUAL_KEY(key as u16);
        input.Anonymous.ki.wScan = unsafe { MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC) } as u16;
        input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

        let written = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        if written != 1 {
            bail!("SendInput failed for key_up");
        }
        Ok(())
    }

    fn scroll(&self, dx: i32, dy: i32) -> Result<()> {
        self.ensure_foreground()?;

        self.check_and_block_input()?;
        defer! {
            let _ = self.unblock_input();
        }

        // 移动光标到目标位置
        let (target_x, target_y) = self.get_target_pos();
        let mut point = POINT {
            x: target_x,
            y: target_y,
        };
        if !self.hwnd.is_invalid() {
            unsafe { ClientToScreen(self.hwnd, &mut point) }.ok()?;
        }
        unsafe { SetCursorPos(point.x, point.y) }?;

        if dy != 0 {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        mouseData: dy as u32,
                        dwFlags: MOUSEEVENTF_WHEEL,
                        ..Default::default()
                    },
                },
            };
            unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        }

        if dx != 0 {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        mouseData: dx as u32,
                        dwFlags: MOUSEEVENTF_HWHEEL,
                        ..Default::default()
                    },
                },
            };
            unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        }

        Ok(())
    }
}

// Win32 句柄（HWND 等）跨线程传递安全（访问时由调用方串行化）。
unsafe impl Send for SeizeInput {}

impl InputBase for SeizeInput {
    fn new(hwnd: HWND, block_input: bool) -> Self {
        Self::new(hwnd, block_input)
    }

    fn touch_down(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        Self::touch_down(self, contact, point.x, point.y)
    }

    fn touch_move(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        Self::touch_move(self, contact, point.x, point.y)
    }

    fn touch_up(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        Self::touch_up(self, contact, point.x, point.y)
    }

    // click 方法使用默认实现

    // swipe 方法使用默认实现

    fn scroll(&mut self, delta: Point2D<i32>) -> Result<()> {
        Self::scroll(self, delta.x, delta.y)
    }

    fn key_down(&mut self, vk_code: i32) -> Result<()> {
        Self::key_down(self, vk_code)
    }

    fn key_up(&mut self, vk_code: i32) -> Result<()> {
        Self::key_up(self, vk_code)
    }

    // press_key 方法使用默认实现
}
