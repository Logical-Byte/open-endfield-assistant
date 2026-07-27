use anyhow::Result;
use windows::Win32::Foundation::HWND;

use super::input_utils::Contact;

pub trait InputBase {
    fn new(hwnd: HWND, block_input: bool) -> Self;

    fn touch_down(&mut self, contact: Contact, x: i32, y: i32) -> Result<()>;

    fn touch_move(&mut self, contact: Contact, x: i32, y: i32) -> Result<()>;

    fn touch_up(&mut self, contact: Contact, x: i32, y: i32) -> Result<()>;

    fn click(&mut self, contact: Contact, x: i32, y: i32) -> Result<()> {
        self.touch_down(contact, x, y)?;
        self.touch_up(contact, x, y)?;
        Ok(())
    }

    fn swipe(&mut self, contact: Contact, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<()> {
        self.touch_down(contact, x1, y1)?;
        self.touch_move(contact, x2, y2)?;
        self.touch_up(contact, x2, y2)?;
        Ok(())
    }
}
