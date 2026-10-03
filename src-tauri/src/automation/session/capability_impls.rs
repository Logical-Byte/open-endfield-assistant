//! `Session` 提供的工作流自动化能力。

use std::{thread, time::Duration};

use anyhow::Result;
use image::{DynamicImage, RgbaImage, imageops};
use imageproc::contrast::ThresholdType;

use crate::{
    automation::{
        AutomationStopped, Clock, Drag, Input, Key, Ocr, Point720p, ScreenCapture, StopToken,
        TemplateMatch, TemplateMatching, TemplateTarget, is_stop_requested,
    },
    platform::input::{Contact, InputBase},
    utils::{point::Point2D, region::Region2D},
    vision::{ocr::text_detection, template_matching},
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

impl Drag for Session {
    fn drag(&mut self, from: Point720p, to: Point720p) -> Result<()> {
        let from = self.resolution_transform.to_physical(from);
        let to = self.resolution_transform.to_physical(to);
        drag_mouse(self.input.as_mut(), &self.stop, from, to)
    }
}

fn drag_mouse(
    input: &mut dyn InputBase,
    stop: &StopToken,
    from: Point2D<i32>,
    to: Point2D<i32>,
) -> Result<()> {
    const STEPS: i32 = 12;
    const STEP_DELAY: Duration = Duration::from_millis(16);

    if is_stop_requested(stop) {
        return Err(AutomationStopped.into());
    }
    input.touch_down(Contact::Left, from)?;
    let mut position = from;
    // 将可中断的移动放在单独的结果中，按下后发生任何错误也必须松开鼠标。
    let movement = (|| -> Result<()> {
        for step in 1..=STEPS {
            thread::sleep(STEP_DELAY);
            if is_stop_requested(stop) {
                return Err(AutomationStopped.into());
            }
            position = Point2D {
                x: from.x + (to.x - from.x) * step / STEPS,
                y: from.y + (to.y - from.y) * step / STEPS,
            };
            input.touch_move(Contact::Left, position)?;
        }
        Ok(())
    })();
    let release = input.touch_up(Contact::Left, position);
    if let Err(error) = &release {
        tracing::warn!(error = %error, "拖动结束后松开鼠标失败");
    }
    movement.and(release)
}

impl TemplateMatching for Session {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        let matched = template_matching::find(
            screenshot,
            target.template_name,
            target.roi,
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
        let output = self.ocr.lock().unwrap().ocr(&cropped)?;
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

impl Clock for Session {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::automation::{new_stop_token, request_stop};

    #[derive(Default)]
    struct RecordingInput {
        pressed: bool,
        moves: Vec<Point2D<i32>>,
        released_at: Option<Point2D<i32>>,
        stop_on_move: Option<StopToken>,
        fail_move: bool,
    }

    impl InputBase for RecordingInput {
        fn touch_down(&mut self, contact: Contact, _point: Point2D<i32>) -> Result<()> {
            assert_eq!(contact, Contact::Left);
            self.pressed = true;
            Ok(())
        }

        fn touch_move(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
            assert_eq!(contact, Contact::Left);
            assert!(self.pressed);
            self.moves.push(point);
            if self.fail_move {
                anyhow::bail!("move failed");
            }
            if let Some(stop) = &self.stop_on_move {
                request_stop(stop);
            }
            Ok(())
        }

        fn touch_up(&mut self, contact: Contact, point: Point2D<i32>) -> Result<()> {
            assert_eq!(contact, Contact::Left);
            assert!(self.pressed);
            self.pressed = false;
            self.released_at = Some(point);
            Ok(())
        }

        fn scroll(&mut self, _delta: Point2D<i32>) -> Result<()> {
            unreachable!()
        }

        fn key_down(&mut self, _vk_code: i32) -> Result<()> {
            unreachable!()
        }

        fn key_up(&mut self, _vk_code: i32) -> Result<()> {
            unreachable!()
        }
    }

    #[test]
    fn drag_moves_in_steps_and_releases_at_destination() {
        let mut input = RecordingInput::default();
        let from = Point2D { x: 100, y: 500 };
        let to = Point2D { x: 300, y: 100 };

        drag_mouse(&mut input, &new_stop_token(), from, to).unwrap();

        assert!(input.moves.len() > 1);
        assert!(input.moves[0].x > from.x && input.moves[0].x < to.x);
        assert!(
            input
                .moves
                .windows(2)
                .all(|pair| { pair[0].x <= pair[1].x && pair[0].y >= pair[1].y })
        );
        assert_eq!(input.moves.last(), Some(&to));
        assert_eq!(input.released_at, Some(to));
        assert!(!input.pressed);
    }

    #[test]
    fn interrupted_drag_releases_the_mouse_without_further_movement() {
        for fail_move in [false, true] {
            let stop = new_stop_token();
            let mut input = RecordingInput {
                stop_on_move: Some(Arc::clone(&stop)),
                fail_move,
                ..Default::default()
            };

            let error = drag_mouse(
                &mut input,
                &stop,
                Point2D { x: 100, y: 500 },
                Point2D { x: 300, y: 100 },
            )
            .unwrap_err();

            assert_eq!(
                error.downcast_ref::<AutomationStopped>().is_none(),
                fail_move
            );
            assert_eq!(input.moves.len(), 1);
            assert_eq!(input.released_at, input.moves.last().copied());
            assert!(!input.pressed);
        }
    }
}
