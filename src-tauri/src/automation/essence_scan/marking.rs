//! 按纯判断计划操作标记，再从新截图确认同一枚基质上的目标状态。

use std::time::Duration;

use anyhow::{Context, Result, ensure};

use crate::{
    automation::{AutomationStopped, Clock, Input, Point720p, ScreenCapture, TemplateMatching},
    essence::{Essence, MarkAction, MarkingPlan},
    navigation, vision,
};

use super::Marking;

const LOCK_BUTTON: Point720p = Point720p { x: 1226, y: 191 };
const ABANDON_BUTTON: Point720p = Point720p { x: 1205, y: 189 };

/// 同时返回可上报的结果和流程结果。调用方须先发布该项，再传播失败或停止。
pub(super) fn apply(
    io: &mut (impl Input + ScreenCapture + TemplateMatching + Clock),
    original: &Essence,
    plan: MarkingPlan,
) -> (Marking, Result<()>) {
    let action = match plan {
        MarkingPlan::Disabled => return (Marking::Disabled, Ok(())),
        MarkingPlan::Skipped => return (Marking::Skipped, Ok(())),
        MarkingPlan::AlreadySet(action) => return (Marking::AlreadySet { action }, Ok(())),
        MarkingPlan::Apply(action) => action,
    };
    match apply_action(io, original, action) {
        Ok(()) => (Marking::Applied { action }, Ok(())),
        Err(error) => {
            let message = if error.downcast_ref::<AutomationStopped>().is_some() {
                "扫描已停止，标记是否生效尚未确认".to_owned()
            } else {
                format!("{error:#}")
            };
            (
                Marking::Failed {
                    action,
                    error: message,
                },
                Err(error),
            )
        }
    }
}

fn apply_action(
    io: &mut (impl Input + ScreenCapture + TemplateMatching + Clock),
    original: &Essence,
    action: MarkAction,
) -> Result<()> {
    // 两个控件都是切换按钮，只点击一次。读回失败时不能用重试猜测按钮状态。
    io.click(match action {
        MarkAction::Lock => LOCK_BUTTON,
        MarkAction::Abandon => ABANDON_BUTTON,
    })
    .context("点击基质标记失败")?;
    io.sleep(Duration::from_millis(300));
    let frame = io.screenshot().context("读取标记后的基质失败")?;
    navigation::require_essence_inventory(io, &frame)?;
    let updated = vision::essence::recognize(io, &frame)?;
    ensure!(
        updated.stats == original.stats
            && updated.levels == original.levels
            && updated.rarity == original.rarity,
        "标记后基质内容发生变化或无法确认，已停止扫描"
    );
    let confirmed = match action {
        MarkAction::Lock => updated.locked == Some(true),
        MarkAction::Abandon => updated.abandoned == Some(true) && updated.locked == Some(false),
    };
    ensure!(confirmed, "未确认目标标记已生效，已停止扫描");
    Ok(())
}
