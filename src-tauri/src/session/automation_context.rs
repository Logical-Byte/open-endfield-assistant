use std::sync::atomic::Ordering;

use anyhow::Result;

use crate::{
    automation::{ActionOutcome, AutomateAction, AutomateExecutor, Key, Point720p},
    session::Session,
    task::TaskStopped,
    utils::point::Point2D,
    windows_ops::input::Contact,
};

/// 借用会话自动化资源的短生命周期解释器门面。
///
/// 上层只通过 [`AutomateExecutor`] 提交符号动作；截图、输入、模板缓存和停止
/// 令牌等实现细节仍由会话持有，并在此上下文中被临时使用。
pub struct AutomateContext<'a> {
    session: &'a mut Session,
}

impl<'a> AutomateContext<'a> {
    pub(super) fn new(session: &'a mut Session) -> Self {
        Self { session }
    }

    fn check_stop(&self) -> Result<()> {
        if self.session.stop.load(Ordering::Relaxed) {
            Err(TaskStopped.into())
        } else {
            Ok(())
        }
    }

    fn click_at(&mut self, point: Point720p) -> Result<()> {
        self.check_stop()?;
        let (x, y) = self.session.resolution.scale_point(point.x, point.y);
        self.session.input.click(
            Contact::Left,
            Point2D {
                x: x as i32,
                y: y as i32,
            },
        )?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.move_mouse_to_safe_position()
    }

    /// 将鼠标移到窗口中心，避免鼠标悬停状态干扰后续识别。
    ///
    /// 该操作供任务开始时主动清理已有鼠标位置使用；普通点击动作完成后也会
    /// 自动执行同样的归位。
    pub fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        let point = Point2D {
            x: self.session.resolution.width as i32 / 2,
            y: self.session.resolution.height as i32 / 2,
        };
        self.session.input.touch_move(Contact::Left, point)
    }

    fn press_key(&mut self, key: Key) -> Result<()> {
        self.check_stop()?;
        let code = match key {
            Key::Escape => 0x1B,
        };
        self.session.input.press_key(code)
    }
}

impl AutomateExecutor for AutomateContext<'_> {
    fn execute(&mut self, action: &AutomateAction) -> Result<ActionOutcome> {
        match action {
            AutomateAction::ClickAt(point) => {
                self.click_at(*point)?;
                Ok(ActionOutcome::Applied)
            }
            AutomateAction::PressKey(key) => {
                self.press_key(*key)?;
                Ok(ActionOutcome::Applied)
            }
            AutomateAction::FindAndClickTemplate(target) => {
                self.check_stop()?;
                let raw = self.session.screencap.screencap()?;
                let frame = self.session.resolution.scale_screenshot_to_base(&raw);
                let matched = self
                    .session
                    .recognition_context(&frame)
                    .find_template_in_roi(target.template_name, target.roi, target.threshold)?;
                let Some(matched) = matched else {
                    return Ok(ActionOutcome::TargetNotFound);
                };
                let center = matched.region.center();
                self.click_at(Point720p {
                    x: center.x,
                    y: center.y,
                })?;
                Ok(ActionOutcome::Applied)
            }
        }
    }
}
