//! [`Capture`](super::Capture) 对各 capability 的计数与转发实现。

use std::time::Duration;

use anyhow::Result;
use image::RgbaImage;

use crate::{
    automation::{
        Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget,
    },
    utils::region::Region2D,
};

use super::Capture;

impl<C: Input + ?Sized> Input for Capture<'_, C> {
    fn click(&mut self, point: Point720p) -> Result<()> {
        self.calls.click += 1;
        self.inner.click(point)
    }

    fn press_key(&mut self, key: Key) -> Result<()> {
        self.calls.press_key += 1;
        self.inner.press_key(key)
    }

    fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        self.calls.move_mouse_to_safe_position += 1;
        self.inner.move_mouse_to_safe_position()
    }
}

impl<C: ScreenCapture + ?Sized> ScreenCapture for Capture<'_, C> {
    fn screenshot(&mut self) -> Result<RgbaImage> {
        self.calls.screenshot += 1;
        self.inner.screenshot()
    }
}

impl<C: TemplateMatching + ?Sized> TemplateMatching for Capture<'_, C> {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        self.calls.find_template += 1;
        self.inner.find_template(screenshot, target)
    }
}

impl<C: Ocr + ?Sized> Ocr for Capture<'_, C> {
    fn recognize_text(
        &mut self,
        screenshot: &RgbaImage,
        region: Region2D<u32>,
    ) -> Result<Option<String>> {
        self.calls.recognize_text += 1;
        self.inner.recognize_text(screenshot, region)
    }
}

impl<C: Clock + ?Sized> Clock for Capture<'_, C> {
    fn sleep(&mut self, duration: Duration) {
        self.calls.sleep += 1;
        self.inner.sleep(duration);
    }
}
