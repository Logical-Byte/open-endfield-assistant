//! 按强策略执行导航动作并验证每次观测。
//!
//! 正常策略中的每一步都会进入 `remaining_hops` 更小的状态。真实 UI 没有响应、识别
//! 抖动或 `transition` 声明遗漏可能破坏这个模型，因此 `executor` 另外限制每个具体 UI
//! 状态最多驱动三次动作。有限状态集合和这个动作上限共同保证异常执行也会终止。

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, bail};
use tracing::{debug, warn};

use crate::automation::{Clock, Input, ScreenCapture, TemplateMatching};

use super::{
    policy::NavigationPolicy,
    recognition::{Recognition, recognize},
    state::UiState,
    transition::Op,
};

/// 同一具体 UI 状态允许重复执行策略动作的最大次数。
const MAX_ACTIONS_PER_STATE: u8 = 3;
/// 一次动作后允许连续出现未识别结果的最大观测次数。
const MAX_UNRECOGNIZED_OBSERVATIONS: u8 = 3;
/// 导航动作后两次 UI 状态观测之间的等待时间。
const OBSERVATION_INTERVAL: Duration = Duration::from_millis(500);

pub(super) struct PolicyExecutor<'a, C> {
    policy: &'a NavigationPolicy,
    cx: &'a mut C,
    action_attempts: HashMap<UiState, u8>,
}

impl<'a, C> PolicyExecutor<'a, C>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    pub(super) fn new(policy: &'a NavigationPolicy, cx: &'a mut C) -> Self {
        Self {
            policy,
            cx,
            action_attempts: HashMap::new(),
        }
    }

    pub(super) fn run(mut self) -> Result<()> {
        let mut current = match recognize(self.cx)? {
            Recognition::Determined(state) => state,
            Recognition::Unrecognized => bail!("当前画面无法识别，无法开始导航"),
        };

        loop {
            if self.policy.is_destination(current) {
                debug!(state = ?current, "导航已抵达目的状态");
                return Ok(());
            }
            let step = self.policy.step(current).ok_or_else(|| {
                anyhow::anyhow!("无法保证从当前具体 UI 状态 {current:?} 抵达目的状态")
            })?;

            let attempts = self.action_attempts.entry(current).or_default();
            if *attempts == MAX_ACTIONS_PER_STATE {
                bail!(
                    "导航未取得进展：具体 UI 状态 {current:?} 已执行策略动作 {MAX_ACTIONS_PER_STATE} 次"
                );
            }
            *attempts += 1;
            debug!(
                state = ?current,
                remaining_hops = step.remaining_hops,
                attempt = *attempts,
                "执行导航策略动作"
            );

            execute_ops(step.transition.ops, self.cx)
                .with_context(|| format!("执行具体 UI 状态 {current:?} 的导航动作失败"))?;
            let observed = observe_after_action(self.cx)?;
            if !step.transition.outcomes.contains(&observed) {
                warn!("导航观测超出跳转声明，尝试从当前状态继续");
                debug!(
                    source = ?current,
                    actual = ?observed,
                    expected = ?step.transition.outcomes,
                    "导航模型违例详情"
                );
                if !self.policy.is_destination(observed) && self.policy.step(observed).is_none() {
                    bail!(
                        "导航模型违例：从 {current:?} 执行动作 {:?} 后观测到未被策略覆盖的 {observed:?}，声明结果为 {:?}",
                        step.transition.ops,
                        step.transition.outcomes,
                    );
                }
            }
            current = observed;
        }
    }
}

fn observe_after_action<C>(cx: &mut C) -> Result<UiState>
where
    C: ScreenCapture + TemplateMatching + Clock,
{
    for _ in 0..MAX_UNRECOGNIZED_OBSERVATIONS {
        cx.sleep(OBSERVATION_INTERVAL);
        if let Recognition::Determined(state) = recognize(cx)? {
            return Ok(state);
        }
    }
    bail!("执行导航动作后连续 {MAX_UNRECOGNIZED_OBSERVATIONS} 次无法识别具体 UI 状态")
}

fn execute_ops<C>(ops: &[Op], cx: &mut C) -> Result<()>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    for (index, op) in ops.iter().enumerate() {
        execute_op(op, cx).with_context(|| format!("导航动作序列的第 {} 步失败", index + 1))?;
    }
    Ok(())
}

fn execute_op<C>(op: &Op, cx: &mut C) -> Result<()>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    match op {
        Op::Click(point) => cx.click(*point),
        Op::PressKey(key) => cx.press_key(*key),
        Op::Sleep(duration) => {
            cx.sleep(*duration);
            Ok(())
        }
        Op::FindAndClickTemplate { target, fallback } => {
            let screenshot = cx.screenshot()?;
            if let Some(matched) = cx.find_template(&screenshot, target)? {
                return cx.click(matched.region.center().into());
            }
            if let Some(point) = fallback {
                return cx.click(*point);
            }
            bail!(
                "画面中没有找到与模板匹配的可点击区域: {}",
                target.template_name
            )
        }
    }
}
