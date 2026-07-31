use std::{thread, time::Duration};

use anyhow::Result;
use windows::Win32::Foundation::HWND;

use super::input_utils::Contact;
use crate::utils::point::Point2D;

pub trait InputBase {
    fn new(hwnd: HWND, block_input: bool) -> Self
    where
        Self: Sized;

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
