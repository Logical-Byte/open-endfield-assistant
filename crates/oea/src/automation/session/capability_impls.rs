//! `Session` 提供的工作流自动化能力。

use std::{thread, time::Duration};

use image::RgbaImage;
use tracing::debug;

use crate::{
    automation::{
        Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget, capabilities,
    },
    platform::input::Contact,
    utils::region::Region2D,
    vision::{ImageRegion, template_matching},
};

use super::Session;

/// 720p 基准坐标中的画面中心，避免鼠标悬停干扰后续识别。
const SAFE_MOUSE_POSITION: Point720p = Point720p { x: 640, y: 360 };

impl ScreenCapture for Session {
    fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
        self.check_stop()?;
        let raw = self.screencap.screencap().map_err(|error| {
            debug!(error = ?error, "游戏截图失败");
            capabilities::Error::CaptureFailed
        })?;
        self.resolution_transform
            .to_canonical_image(raw)
            .map_err(capabilities::Error::from)
    }
}

impl Input for Session {
    fn click(&mut self, point: Point720p) -> Result<(), capabilities::Error> {
        self.check_stop()?;
        let point = self.resolution_transform.to_physical(point);
        self.input.click(Contact::Left, point).map_err(|error| {
            debug!(?point, error = ?error, "点击游戏窗口失败");
            capabilities::Error::ExecutionFailed
        })?;
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()
    }

    fn press_key(&mut self, key: Key) -> Result<(), capabilities::Error> {
        self.check_stop()?;
        let vk_code = match key {
            Key::Escape => 0x1B,
        };
        self.input.press_key(vk_code).map_err(|error| {
            debug!(?key, error = ?error, "发送游戏按键失败");
            capabilities::Error::ExecutionFailed
        })
    }

    fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
        let point = self.resolution_transform.to_physical(SAFE_MOUSE_POSITION);
        self.input
            .touch_move(Contact::Left, point)
            .map_err(|error| {
                debug!(error = ?error, "移动游戏鼠标失败");
                capabilities::Error::ExecutionFailed
            })
    }
}

impl TemplateMatching for Session {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>, capabilities::Error> {
        let matched = template_matching::find(
            &ImageRegion::new(screenshot, target.roi).map_err(|error| {
                debug!(roi = ?target.roi, error = ?error, "模板匹配区域无效");
                capabilities::Error::ExecutionFailed
            })?,
            target.template_name,
            &mut self.templates,
        )
        .map_err(|error| {
            debug!(template = target.template_name, error = ?error, "模板匹配失败");
            capabilities::Error::ExecutionFailed
        })?;
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
    ) -> Result<Option<String>, capabilities::Error> {
        Ok(self
            .ocr
            .lock()
            .unwrap()
            .recognize_region(&ImageRegion::new(screenshot, region).map_err(|error| {
                debug!(?region, error = ?error, "OCR 区域无效");
                capabilities::Error::ExecutionFailed
            })?)
            .map_err(|error| {
                debug!(?region, error = ?error, "OCR 识别失败");
                capabilities::Error::ExecutionFailed
            })?
            .map(|result| result.text))
    }
}

impl Clock for Session {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}
