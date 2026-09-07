//! 声明并执行场景之间的固定操作序列。

use std::time::Duration;

use anyhow::{Context, Result};

use crate::automation::{
    Clock, Input, Key, Point720p, ScreenCapture, TemplateMatching, TemplateTarget,
};

use super::SceneId;

#[derive(Debug, Clone, Copy, PartialEq)]
/// 场景跳转专用的有限操作词汇。
pub enum Op {
    Click(Point720p),
    PressKey(Key),
    Sleep(Duration),
    FindAndClickTemplate(TemplateTarget),
}

/// 到目标场景的有序操作序列。
///
/// 场景声明负责拥有操作列表；路由执行只选择目标并解释该列表。复杂的分支、循环和
/// 重试继续由业务工作流负责，避免把本类型扩展成通用自动化语言。
pub struct Transition<'a> {
    /// 操作完成后应到达的场景。
    pub target: SceneId,
    /// 按顺序执行的操作。
    pub ops: &'a [Op],
}

impl Transition<'_> {
    /// 使用工作流能力顺序执行操作，并为首个失败补充导航位置上下文。
    pub fn execute<C>(&self, cx: &mut C) -> Result<()>
    where
        C: ScreenCapture + Input + TemplateMatching + Clock + ?Sized,
    {
        for (index, op) in self.ops.iter().enumerate() {
            execute_op(op, cx).with_context(|| {
                let ops = self
                    .ops
                    .iter()
                    .enumerate()
                    .map(|(index, op)| format!("{}. {op:?}", index + 1))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "跳转操作出错：跳转到 {:?}，行动序列 {ops}，第 {} 步出错",
                    self.target,
                    index + 1
                )
            })?;
        }
        Ok(())
    }
}

fn execute_op<C>(op: &Op, cx: &mut C) -> Result<()>
where
    C: ScreenCapture + Input + TemplateMatching + Clock + ?Sized,
{
    match op {
        Op::Click(point) => cx.click(*point),
        Op::PressKey(key) => cx.press_key(*key),
        Op::Sleep(duration) => {
            cx.sleep(*duration);
            Ok(())
        }
        Op::FindAndClickTemplate(target) => {
            let screenshot = cx.screenshot()?;
            let matched = cx.find_template(&screenshot, target)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "画面中没有找到与模板匹配的可点击区域: {}",
                    target.template_name
                )
            })?;
            cx.click(matched.region.center().into())
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
            Clock, Input, Key, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
            TemplateTarget,
        },
        scene::{
            SceneId,
            transition::{Op, Transition},
        },
        utils::region::Region2D,
    };

    #[derive(Debug, PartialEq)]
    enum Call {
        Click(Point720p),
        PressKey(Key),
        Sleep(Duration),
        Screenshot,
        FindTemplate(TemplateTarget),
    }

    struct MockAutomation {
        calls: Vec<Call>,
        template_match: Option<TemplateMatch>,
        fail_click: bool,
    }

    #[derive(Debug)]
    struct MockInputError;

    impl std::fmt::Display for MockInputError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("mock input error")
        }
    }

    impl std::error::Error for MockInputError {}

    impl MockAutomation {
        fn successful() -> Self {
            Self {
                calls: Vec::new(),
                template_match: Some(TemplateMatch {
                    region: Region2D::from_ltrb(10, 20, 30, 40),
                    score: 0.9,
                }),
                fail_click: false,
            }
        }
    }

    impl ScreenCapture for MockAutomation {
        fn screenshot(&mut self) -> Result<RgbaImage> {
            self.calls.push(Call::Screenshot);
            Ok(RgbaImage::new(1280, 720))
        }
    }

    impl Input for MockAutomation {
        fn click(&mut self, point: Point720p) -> Result<()> {
            self.calls.push(Call::Click(point));
            if self.fail_click {
                return Err(MockInputError.into());
            }
            Ok(())
        }

        fn press_key(&mut self, key: Key) -> Result<()> {
            self.calls.push(Call::PressKey(key));
            Ok(())
        }

        fn move_mouse_to_safe_position(&mut self) -> Result<()> {
            Ok(())
        }
    }

    impl TemplateMatching for MockAutomation {
        fn find_template(
            &mut self,
            _screenshot: &RgbaImage,
            target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>> {
            self.calls.push(Call::FindTemplate(*target));
            Ok(self.template_match)
        }
    }

    impl Clock for MockAutomation {
        fn sleep(&mut self, duration: Duration) {
            self.calls.push(Call::Sleep(duration));
        }
    }

    const TARGET: TemplateTarget = TemplateTarget {
        template_name: "button.png",
        roi: Region2D::from_ltrb(1, 2, 101, 102),
        threshold: 0.75,
    };

    #[test]
    fn executes_operations_in_order_with_their_parameters() {
        let ops = [
            Op::Click(Point720p { x: 3, y: 4 }),
            Op::PressKey(Key::Escape),
            Op::Sleep(Duration::from_millis(250)),
            Op::FindAndClickTemplate(TARGET),
        ];
        let transition = Transition {
            target: SceneId::协议终端,
            ops: &ops,
        };
        let mut cx = MockAutomation::successful();

        transition.execute(&mut cx).unwrap();

        assert_eq!(
            cx.calls,
            [
                Call::Click(Point720p { x: 3, y: 4 }),
                Call::PressKey(Key::Escape),
                Call::Sleep(Duration::from_millis(250)),
                Call::Screenshot,
                Call::FindTemplate(TARGET),
                Call::Click(Point720p { x: 20, y: 30 }),
            ]
        );
    }

    #[test]
    fn does_not_click_when_the_template_is_absent() {
        let ops = [Op::FindAndClickTemplate(TARGET)];
        let transition = Transition {
            target: SceneId::档案库主界面,
            ops: &ops,
        };
        let mut cx = MockAutomation::successful();
        cx.template_match = None;

        assert!(transition.execute(&mut cx).is_err());
        assert_eq!(cx.calls, [Call::Screenshot, Call::FindTemplate(TARGET)]);
    }

    #[test]
    fn stops_at_the_first_external_error_and_preserves_its_source() {
        let ops = [
            Op::Click(Point720p { x: 7, y: 8 }),
            Op::PressKey(Key::Escape),
        ];
        let transition = Transition {
            target: SceneId::协议终端,
            ops: &ops,
        };
        let mut cx = MockAutomation::successful();
        cx.fail_click = true;

        let error = transition.execute(&mut cx).unwrap_err();

        assert!(error.downcast_ref::<MockInputError>().is_some());
        assert_eq!(cx.calls, [Call::Click(Point720p { x: 7, y: 8 })]);
    }
}
