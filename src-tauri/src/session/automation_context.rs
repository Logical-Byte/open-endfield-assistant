//! 将工作流自动化能力适配到生产 `Session` 组件。

use std::{thread, time::Duration};

use anyhow::Result;
use image::{DynamicImage, RgbaImage, imageops};
use imageproc::contrast::ThresholdType;

use crate::{
    automation::{
        Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget,
    },
    ocr::text_detection,
    template_matching::{TemplateSource, match_template_in_region},
    utils::{point::Point2D, region::Region2D},
    windows_ops::input::Contact,
};

use super::Session;

/// 工作流自动化能力的生产实现注入点。
///
/// 该上下文短暂借用一个 [`Session`]，并把能力调用委托给会话持有的截图、输入、
/// 模板和 OCR 组件。它不拥有另一套自动化实现，也不向工作流暴露会话内部结构。
pub struct AutomationContext<'a> {
    session: &'a mut Session,
}

impl<'a> AutomationContext<'a> {
    pub(super) fn new(session: &'a mut Session) -> Self {
        Self { session }
    }
}

impl ScreenCapture for AutomationContext<'_> {
    fn screenshot(&mut self) -> Result<RgbaImage> {
        self.session.check_stop()?;
        let raw = self.session.screencap.screencap()?;
        Ok(self.session.resolution.scale_screenshot_to_base(&raw))
    }
}

impl Input for AutomationContext<'_> {
    fn click(&mut self, point: Point720p) -> Result<()> {
        self.session.check_stop()?;
        let (x, y) = self.session.resolution.scale_point(point.x, point.y);
        self.session.input.click(
            Contact::Left,
            Point2D {
                x: x as i32,
                y: y as i32,
            },
        )?;
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()
    }

    fn press_key(&mut self, key: Key) -> Result<()> {
        self.session.check_stop()?;
        let vk_code = match key {
            Key::Escape => 0x1B,
        };
        self.session.input.press_key(vk_code)
    }

    fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        let point = Point2D {
            x: self.session.resolution.width as i32 / 2,
            y: self.session.resolution.height as i32 / 2,
        };
        self.session.input.touch_move(Contact::Left, point)
    }
}

impl TemplateMatching for AutomationContext<'_> {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        let template = self.session.templates.get(target.template_name)?;
        let matched = match_template_in_region(screenshot, template, Some(target.roi))?;
        Ok(
            (matched.score >= target.threshold).then_some(TemplateMatch {
                region: matched.region,
                score: matched.score,
            }),
        )
    }
}

impl Ocr for AutomationContext<'_> {
    fn recognize_text(
        &mut self,
        screenshot: &RgbaImage,
        region: Region2D<u32>,
    ) -> Result<Option<String>> {
        let cropped = imageops::crop_imm(
            screenshot,
            region.x0(),
            region.y0(),
            region.width(),
            region.height(),
        )
        .to_image();
        let rgb = DynamicImage::ImageRgba8(cropped).to_rgb8();
        let Some(text_region) =
            text_detection::detect_single_line(&rgb, 128, ThresholdType::Binary, 6)
        else {
            return Ok(None);
        };
        let cropped = imageops::crop_imm(
            &rgb,
            text_region.x0(),
            text_region.y0(),
            text_region.width(),
            text_region.height(),
        )
        .to_image();
        let output = self.session.ocr.lock().unwrap().ocr(&cropped)?;
        Ok(Some(
            output
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ))
    }
}

impl Clock for AutomationContext<'_> {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}
