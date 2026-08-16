//! 游戏窗口输入接口。

use std::{thread, time::Duration};

use anyhow::Result;

use crate::utils::point::Point2D;

use super::{WindowHandle, details};

/// 鼠标按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    /// 鼠标左键。
    Left = 0,
    /// 鼠标右键。
    Right = 1,
    /// 鼠标中键。
    Middle = 2,
    /// 第一个扩展鼠标键。
    X1 = 3,
    /// 第二个扩展鼠标键。
    X2 = 4,
}

/// 可替换的窗口输入能力。
pub trait InputBase: Send {
    fn touch_down(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()>;
    fn touch_move(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()>;
    fn touch_up(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()>;

    fn click(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        self.touch_down(contact, point)?;
        thread::sleep(Duration::from_millis(10));
        self.touch_up(contact, point)?;
        Ok(())
    }

    fn swipe(&mut self, contact: Contact, p0: Point2D<i32>, p1: Point2D<i32>) -> Result<()> {
        self.touch_down(contact, p0)?;
        self.touch_move(contact, p1)?;
        self.touch_up(contact, p1)?;
        Ok(())
    }

    fn scroll(&mut self, delta: Point2D<i32>) -> Result<()>;
    fn key_down(&mut self, vk_code: i32) -> Result<()>;
    fn key_up(&mut self, vk_code: i32) -> Result<()>;

    fn press_key(&mut self, vk_code: i32) -> Result<()> {
        self.key_down(vk_code)?;
        thread::sleep(Duration::from_millis(10));
        self.key_up(vk_code)?;
        Ok(())
    }
}

/// 使用 Windows 物理输入接口操作游戏窗口的输入器。
pub struct SeizeInput {
    state: details::input::SeizeInputState,
}

impl SeizeInput {
    /// 绑定目标窗口，并配置操作期间是否屏蔽真实输入。
    pub fn new(window: WindowHandle, block_input: bool) -> Self {
        Self {
            state: details::input::SeizeInputState::new(window, block_input),
        }
    }
}

// 原生窗口句柄跨线程传递安全；输入器由调用方串行访问。
unsafe impl Send for SeizeInput {}

impl InputBase for SeizeInput {
    fn touch_down(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        self.state.touch_down(contact, point.x, point.y)
    }

    fn touch_move(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        self.state.touch_move(contact, point.x, point.y)
    }

    fn touch_up(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
        self.state.touch_up(contact, point.x, point.y)
    }

    fn scroll(&mut self, delta: Point2D<i32>) -> Result<()> {
        self.state.scroll(delta.x, delta.y)
    }

    fn key_down(&mut self, vk_code: i32) -> Result<()> {
        self.state.key_down(vk_code)
    }

    fn key_up(&mut self, vk_code: i32) -> Result<()> {
        self.state.key_up(vk_code)
    }
}
