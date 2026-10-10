//! `Session` 提供的工作流自动化能力。

use std::{thread, time::Duration};

use anyhow::Result;
use image::RgbaImage;

use crate::{
    automation::{
        Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget,
    },
    platform::input::Contact,
    utils::region::Region2D,
    vision::{ImageRegion, template_matching},
};

use super::Session;

/// 720p 基准坐标中的画面中心，避免鼠标悬停干扰后续识别。
const SAFE_MOUSE_POSITION: Point720p = Point720p { x: 640, y: 360 };

impl ScreenCapture for Session {
    fn screenshot(&mut self) -> Result<RgbaImage> {
        self.check_stop()?;
        let raw = self.screencap.screencap()?;
        self.resolution_transform.to_canonical_image(raw)
    }
}

impl Input for Session {
    fn click(&mut self, point: Point720p) -> Result<()> {
        self.check_stop()?;
        let point = self.resolution_transform.to_physical(point);
        self.input.click(Contact::Left, point)?;
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()
    }

    fn press_key(&mut self, key: Key) -> Result<()> {
        self.check_stop()?;
        let vk_code = match key {
            Key::Escape => 0x1B,
        };
        self.input.press_key(vk_code)
    }

    fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        let point = self.resolution_transform.to_physical(SAFE_MOUSE_POSITION);
        self.input.touch_move(Contact::Left, point)
    }
}

impl TemplateMatching for Session {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        let matched = template_matching::find(
            &ImageRegion::new(screenshot, target.roi)?,
            target.template_name,
            &mut self.templates,
        )?;
        Ok(
            (matched.score >= target.threshold).then_some(TemplateMatch {
                region: matched.region,
                score: matched.score,
            }),
        )
    }
}

impl Ocr for Session {
    fn recognize_text(
        &mut self,
        screenshot: &RgbaImage,
        region: Region2D<u32>,
    ) -> Result<Option<String>> {
        Ok(self
            .ocr
            .lock()
            .unwrap()
            .recognize_region(&ImageRegion::new(screenshot, region)?)?
            .map(|result| result.text))
    }
}

impl Clock for Session {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}
