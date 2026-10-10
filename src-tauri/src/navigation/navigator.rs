//! 具体 UI 状态导航的 crate 内门面。

use crate::navigation;
use tracing::debug;

use crate::automation::{Clock, Input, ScreenCapture, TemplateMatching};

use super::{
    executor::PolicyExecutor, graph::NavigationGraph, policy::NavigationPolicy, state::UiState,
};

/// 识别当前具体 UI 状态并执行强策略的导航器。
pub(crate) struct Navigator {
    graph: NavigationGraph,
}

impl Navigator {
    /// 使用游戏当前版本的静态 `transition` 声明构造导航器。
    pub(crate) fn new() -> Self {
        Self {
            graph: NavigationGraph::production(),
        }
    }

    /// 从当前可识别状态导航到一个具体 UI 状态。
    pub(crate) fn navigate_to<C>(
        &self,
        target: UiState,
        cx: &mut C,
    ) -> Result<(), navigation::Error>
    where
        C: ScreenCapture + Input + TemplateMatching + Clock,
    {
        let policy = NavigationPolicy::build(&self.graph, std::iter::once(target))
            .inspect_err(|error| debug!(target = ?target, error = ?error, "构建导航策略失败"))?;
        debug!(target = ?target, "开始导航到具体 UI 状态");
        PolicyExecutor::new(&policy, cx)
            .run()
            .inspect_err(|error| {
                if !error.is_stopped_by_user() {
                    debug!(target = ?target, error = ?error, "导航失败");
                }
            })?;
        debug!(target = ?target, "导航完成");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::automation::{self, capabilities};
    use image::RgbaImage;

    use crate::{
        automation::{
            Clock, Input, Key, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
            TemplateTarget,
        },
        navigation::{
            self, Navigator,
            state::{ArchiveSubscene, RecordsPage, UiState},
        },
    };

    #[derive(Clone, Copy)]
    enum VisibleUi {
        /// 测试边界当前显示大世界。
        Overworld,
        /// 测试边界当前显示协议终端。
        Terminal,
    }

    struct GameBoundary {
        visible: VisibleUi,
        escape_changes_ui: bool,
        escape_presses: u8,
        unrecognized_frames_after_escape: u8,
        unrecognized_frames_remaining: u8,
        frame_recognizable: bool,
    }

    impl ScreenCapture for GameBoundary {
        fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
            self.frame_recognizable = self.unrecognized_frames_remaining == 0;
            self.unrecognized_frames_remaining =
                self.unrecognized_frames_remaining.saturating_sub(1);
            Ok(RgbaImage::new(1280, 720))
        }
    }

    impl Input for GameBoundary {
        fn click(&mut self, _point: Point720p) -> Result<(), capabilities::Error> {
            Ok(())
        }

        fn press_key(&mut self, key: Key) -> Result<(), capabilities::Error> {
            assert_eq!(key, Key::Escape);
            self.escape_presses += 1;
            if self.escape_changes_ui {
                self.visible = VisibleUi::Terminal;
                self.unrecognized_frames_remaining = self.unrecognized_frames_after_escape;
            }
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
            Ok(())
        }
    }

    impl TemplateMatching for GameBoundary {
        fn find_template(
            &mut self,
            _screenshot: &RgbaImage,
            target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>, capabilities::Error> {
            let matches = self.frame_recognizable
                && matches!(
                    (self.visible, target.template_name),
                    (VisibleUi::Overworld, "协议终端.png") | (VisibleUi::Terminal, "档案库.png")
                );
            Ok(matches.then_some(TemplateMatch {
                region: target.roi,
                score: 1.0,
            }))
        }
    }

    impl Clock for GameBoundary {
        fn sleep(&mut self, _duration: Duration) {}
    }

    #[derive(Clone, Copy)]
    enum ArchiveVisibleUi {
        /// 测试边界当前显示档案详情页面。
        Detail,
        /// 测试边界当前显示“见闻辑录 - 纸质记录”。
        Paper,
    }

    struct DetailFallbackBoundary {
        visible: ArchiveVisibleUi,
        detail_close_matches: u8,
        clicks: Vec<Point720p>,
    }

    impl ScreenCapture for DetailFallbackBoundary {
        fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
            Ok(RgbaImage::new(1280, 720))
        }
    }

    impl Input for DetailFallbackBoundary {
        fn click(&mut self, point: Point720p) -> Result<(), capabilities::Error> {
            self.clicks.push(point);
            if point == (Point720p { x: 1240, y: 50 }) {
                self.visible = ArchiveVisibleUi::Paper;
            }
            Ok(())
        }

        fn press_key(&mut self, _key: Key) -> Result<(), capabilities::Error> {
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
            Ok(())
        }
    }

    impl TemplateMatching for DetailFallbackBoundary {
        fn find_template(
            &mut self,
            _screenshot: &RgbaImage,
            target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>, capabilities::Error> {
            let matches = match (self.visible, target.template_name) {
                (ArchiveVisibleUi::Detail, "情报档案库/档案详情装饰.png") => true,
                (ArchiveVisibleUi::Detail, "情报档案库/档案详情关闭.png") => {
                    let matches = self.detail_close_matches == 0;
                    self.detail_close_matches += 1;
                    matches
                }
                (ArchiveVisibleUi::Paper, "情报档案库/情报档案库标题.png")
                | (ArchiveVisibleUi::Paper, "情报档案库/档案库子界面关闭.png")
                | (ArchiveVisibleUi::Paper, "情报档案库/见闻辑录水印.png") => true,
                _ => false,
            };
            Ok(matches.then_some(TemplateMatch {
                region: target.roi,
                score: 1.0,
            }))
        }
    }

    impl Clock for DetailFallbackBoundary {
        fn sleep(&mut self, _duration: Duration) {}
    }

    #[test]
    fn navigates_from_overworld_to_terminal() {
        let navigator = Navigator::new();
        let mut game = GameBoundary {
            visible: VisibleUi::Overworld,
            escape_changes_ui: true,
            escape_presses: 0,
            unrecognized_frames_after_escape: 0,
            unrecognized_frames_remaining: 0,
            frame_recognizable: true,
        };

        navigator.navigate_to(UiState::Terminal, &mut game).unwrap();

        assert_eq!(game.escape_presses, 1);
    }

    #[test]
    fn stops_after_the_same_state_drives_three_actions() {
        let navigator = Navigator::new();
        let mut game = GameBoundary {
            visible: VisibleUi::Overworld,
            escape_changes_ui: false,
            escape_presses: 0,
            unrecognized_frames_after_escape: 0,
            unrecognized_frames_remaining: 0,
            frame_recognizable: true,
        };

        let error = navigator
            .navigate_to(UiState::Terminal, &mut game)
            .unwrap_err();

        assert_eq!(game.escape_presses, 3);
        assert!(error.to_string().contains("已执行策略动作 3 次"));
        assert_eq!(
            serde_json::to_value(automation::WorkerError::from(error)).unwrap(),
            serde_json::json!({ "kind": "navigationFailed" })
        );
    }

    #[test]
    fn retries_unrecognized_frames_without_repeating_the_action() {
        let navigator = Navigator::new();
        let mut game = GameBoundary {
            visible: VisibleUi::Overworld,
            escape_changes_ui: true,
            escape_presses: 0,
            unrecognized_frames_after_escape: 2,
            unrecognized_frames_remaining: 0,
            frame_recognizable: true,
        };

        navigator.navigate_to(UiState::Terminal, &mut game).unwrap();

        assert_eq!(game.escape_presses, 1);
    }

    #[test]
    fn archive_detail_close_uses_the_coordinate_fallback() {
        let navigator = Navigator::new();
        let mut game = DetailFallbackBoundary {
            visible: ArchiveVisibleUi::Detail,
            detail_close_matches: 0,
            clicks: Vec::new(),
        };

        navigator
            .navigate_to(
                UiState::archive_subscene(ArchiveSubscene::Records(RecordsPage::Paper)),
                &mut game,
            )
            .unwrap();

        assert_eq!(game.clicks, [Point720p { x: 1240, y: 50 }]);
    }
    #[test]
    fn preserves_stop_and_capture_failure_during_navigation() {
        struct FailedCapture {
            stopped: bool,
        }
        impl ScreenCapture for FailedCapture {
            fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
                if self.stopped {
                    Err(capabilities::Error::StoppedByUser)
                } else {
                    Err(capabilities::Error::CaptureFailed)
                }
            }
        }
        impl Input for FailedCapture {
            fn click(&mut self, _: Point720p) -> Result<(), capabilities::Error> {
                unreachable!()
            }
            fn press_key(&mut self, _: Key) -> Result<(), capabilities::Error> {
                unreachable!()
            }
            fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
                unreachable!()
            }
        }
        impl TemplateMatching for FailedCapture {
            fn find_template(
                &mut self,
                _: &RgbaImage,
                _: &TemplateTarget,
            ) -> Result<Option<TemplateMatch>, capabilities::Error> {
                unreachable!()
            }
        }
        impl Clock for FailedCapture {
            fn sleep(&mut self, _: Duration) {
                unreachable!()
            }
        }

        let navigator = Navigator::new();
        let stopped = navigator
            .navigate_to(UiState::Terminal, &mut FailedCapture { stopped: true })
            .unwrap_err();
        assert!(stopped.is_stopped_by_user());
        let failed = navigator
            .navigate_to(UiState::Terminal, &mut FailedCapture { stopped: false })
            .unwrap_err();
        assert!(matches!(
            &failed,
            navigation::Error::Capability(capabilities::Error::CaptureFailed)
        ));
        assert!(!failed.is_stopped_by_user());
    }
}
