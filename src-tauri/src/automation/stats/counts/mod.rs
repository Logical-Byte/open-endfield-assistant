//! capability 调用次数捕获。

mod capability_impls;

use std::time::{Duration, Instant};

/// 一段 capability 调用区间的计数结果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapabilityCallCounts {
    pub screenshot: u64,
    pub click: u64,
    pub press_key: u64,
    pub move_mouse_to_safe_position: u64,
    pub find_template: u64,
    pub recognize_text: u64,
    pub sleep: u64,
}

/// 一段捕获区间结束后的统计摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureSummary {
    pub elapsed: Duration,
    pub calls: CapabilityCallCounts,
}

/// 为任意 capability 实现增加区间计数的 adapter。
pub struct Capture<'a, C: ?Sized> {
    pub(super) inner: &'a mut C,
    started_at: Instant,
    pub(super) calls: CapabilityCallCounts,
    finished: bool,
}

impl<'a, C: ?Sized> Capture<'a, C> {
    pub fn new(inner: &'a mut C) -> Self {
        Self {
            inner,
            started_at: Instant::now(),
            calls: CapabilityCallCounts::default(),
            finished: false,
        }
    }

    pub fn finish(mut self) -> CaptureSummary {
        self.finished = true;
        CaptureSummary {
            elapsed: self.started_at.elapsed(),
            calls: self.calls,
        }
    }
}

impl<C: ?Sized> Drop for Capture<'_, C> {
    fn drop(&mut self) {
        if !self.finished {
            tracing::warn!("capability 计数捕获未调用 finish，丢弃未完成的统计结果");
        }
    }
}

#[cfg(test)]
mod tests {
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

    use super::{CapabilityCallCounts, Capture};

    #[derive(Default)]
    struct InputOnly;

    impl Input for InputOnly {
        fn click(&mut self, _point: Point720p) -> Result<()> {
            Ok(())
        }

        fn press_key(&mut self, _key: Key) -> Result<()> {
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn captures_a_partial_capability_set() {
        let mut input = InputOnly;

        let mut captured = Capture::new(&mut input);
        captured.click(Point720p { x: 10, y: 20 }).unwrap();
        captured.press_key(Key::Escape).unwrap();
        captured.move_mouse_to_safe_position().unwrap();
        let summary = captured.finish();

        assert_eq!(summary.calls.click, 1);
        assert_eq!(summary.calls.press_key, 1);
        assert_eq!(summary.calls.move_mouse_to_safe_position, 1);
        assert_eq!(summary.calls.screenshot, 0);
        assert_eq!(summary.calls.find_template, 0);
        assert_eq!(summary.calls.recognize_text, 0);
        assert_eq!(summary.calls.sleep, 0);
    }

    #[derive(Default)]
    struct AllCapabilities;

    impl ScreenCapture for AllCapabilities {
        fn screenshot(&mut self) -> Result<RgbaImage> {
            Ok(RgbaImage::new(1, 1))
        }
    }

    impl Input for AllCapabilities {
        fn click(&mut self, _point: Point720p) -> Result<()> {
            Ok(())
        }

        fn press_key(&mut self, _key: Key) -> Result<()> {
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<()> {
            Ok(())
        }
    }

    impl TemplateMatching for AllCapabilities {
        fn find_template(
            &mut self,
            _screenshot: &RgbaImage,
            _target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>> {
            Ok(None)
        }
    }

    impl Ocr for AllCapabilities {
        fn recognize_text(
            &mut self,
            _screenshot: &RgbaImage,
            _region: Region2D<u32>,
        ) -> Result<Option<String>> {
            Ok(None)
        }
    }

    impl Clock for AllCapabilities {
        fn sleep(&mut self, _duration: Duration) {}
    }

    #[test]
    fn counts_each_capability_call() {
        let mut capabilities = AllCapabilities;
        let target = TemplateTarget {
            template_name: "test",
            roi: Region2D::from_ltrb(0, 0, 1, 1),
            threshold: 0.9,
        };

        let mut captured = Capture::new(&mut capabilities);
        let screenshot = captured.screenshot().unwrap();
        captured.click(Point720p { x: 10, y: 20 }).unwrap();
        captured.press_key(Key::Escape).unwrap();
        captured.move_mouse_to_safe_position().unwrap();
        captured.find_template(&screenshot, &target).unwrap();
        captured
            .recognize_text(&screenshot, Region2D::from_ltrb(0, 0, 1, 1))
            .unwrap();
        captured.sleep(Duration::ZERO);
        let summary = captured.finish();

        assert_eq!(
            summary.calls,
            CapabilityCallCounts {
                screenshot: 1,
                click: 1,
                press_key: 1,
                move_mouse_to_safe_position: 1,
                find_template: 1,
                recognize_text: 1,
                sleep: 1,
            }
        );
    }

    struct FailingScreenCapture;

    impl ScreenCapture for FailingScreenCapture {
        fn screenshot(&mut self) -> Result<RgbaImage> {
            anyhow::bail!("screenshot failed")
        }
    }

    #[test]
    fn counts_a_capability_call_that_returns_an_error() {
        let mut screen_capture = FailingScreenCapture;

        let mut captured = Capture::new(&mut screen_capture);
        let result = captured.screenshot();
        let summary = captured.finish();

        assert!(result.is_err());
        assert_eq!(summary.calls.screenshot, 1);
    }

    #[derive(Default)]
    struct InputWithInternalMove {
        safe_moves: u64,
    }

    impl Input for InputWithInternalMove {
        fn click(&mut self, _point: Point720p) -> Result<()> {
            self.move_mouse_to_safe_position()
        }

        fn press_key(&mut self, _key: Key) -> Result<()> {
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<()> {
            self.safe_moves += 1;
            Ok(())
        }
    }

    #[test]
    fn does_not_count_helper_work_inside_the_wrapped_capability() {
        let mut input = InputWithInternalMove::default();

        let mut captured = Capture::new(&mut input);
        captured.click(Point720p { x: 10, y: 20 }).unwrap();
        let summary = captured.finish();

        assert_eq!(summary.calls.click, 1);
        assert_eq!(summary.calls.move_mouse_to_safe_position, 0);
        assert_eq!(input.safe_moves, 1);
    }

    #[test]
    fn nested_captures_include_the_call_in_both_summaries() {
        let mut input = InputOnly;

        let mut parent = Capture::new(&mut input);
        let mut child = Capture::new(&mut parent);
        child.click(Point720p { x: 10, y: 20 }).unwrap();
        let child_summary = child.finish();
        let parent_summary = parent.finish();

        assert_eq!(child_summary.calls.click, 1);
        assert_eq!(parent_summary.calls.click, 1);
    }
}
