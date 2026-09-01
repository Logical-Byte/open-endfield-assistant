//! Execution and verification of a single planned route.

use std::{thread, time::Duration};

use anyhow::Result;
use tracing::{debug, warn};

use super::{model::SceneId, route_planner::Route, scene_detector::SceneDetector};
use crate::session::Session;

/// 一次路由执行的可恢复结果；`Err` 仅表示动作或场景检测本身出错。
#[derive(Clone, Copy)]
pub(super) enum RouteExecutionOutcome {
    RouteFinished { final_scene: SceneId },
    NeedsReplan { current_scene: SceneId },
}

impl RouteExecutionOutcome {
    pub(super) fn observed_scene(self) -> SceneId {
        match self {
            Self::RouteFinished { final_scene } => final_scene,
            Self::NeedsReplan { current_scene } => current_scene,
        }
    }
}

/// 执行一条既定路由，并负责每一步的验证与重试。
pub(super) struct RouteExecutor<'a> {
    scene_detector: &'a SceneDetector,
    session: &'a mut Session,
    route: Route,
}

impl<'a> RouteExecutor<'a> {
    pub(super) fn new(
        scene_detector: &'a SceneDetector,
        session: &'a mut Session,
        route: Route,
    ) -> Self {
        Self {
            scene_detector,
            session,
            route,
        }
    }

    pub(super) fn run(mut self) -> Result<RouteExecutionOutcome> {
        const MAX_RETRIES_PER_STEP: u32 = 3;

        let mut current_scene = self.route.source;
        for index in 0..self.route.steps.len() {
            let (from, to) = self.route.steps[index];
            debug!(
                "导航步骤 {}/{}: {:?} → {:?}",
                index + 1,
                self.route.steps.len(),
                from,
                to
            );

            self.execute_single_step(from, to)?;

            let mut step_ok = false;
            for retry in 0..MAX_RETRIES_PER_STEP {
                thread::sleep(Duration::from_millis(500));
                let after = self.scene_detector.detect_current_scene(self.session)?;
                current_scene = after;
                // 关闭档案详情会返回进入详情前的任意档案库子界面。
                let arrived = if from == SceneId::档案详情页面 {
                    matches!(after, SceneId::档案库子界面(_))
                } else {
                    after == to
                };
                if arrived {
                    debug!("导航步骤 {index} 成功，已到达 {:?}", to);
                    step_ok = true;
                    break;
                }
                if retry < MAX_RETRIES_PER_STEP - 1 {
                    warn!(
                        "导航步骤 {index}: 预期到达 {:?}，实际检测到 {:?}，重试 ({}/{})",
                        to,
                        after,
                        retry + 1,
                        MAX_RETRIES_PER_STEP
                    );
                    self.execute_single_step(from, to)?;
                }
            }

            if !step_ok {
                return Ok(RouteExecutionOutcome::NeedsReplan { current_scene });
            }
        }

        Ok(RouteExecutionOutcome::RouteFinished {
            final_scene: current_scene,
        })
    }

    fn execute_single_step(&mut self, from: SceneId, to: SceneId) -> Result<()> {
        let scene = self
            .scene_detector
            .get_registered_scene(from)
            .ok_or_else(|| anyhow::anyhow!("未注册的场景: {:?}", from))?;
        scene.execute_transition(to, self.session)
    }
}
