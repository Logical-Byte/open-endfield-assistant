//! 场景管理器。
//!
//! 提供：
//! 1. 场景检测：自动判断当前处于哪个游戏界面
//! 2. 场景导航：从任意受支持场景自动跳转到目标场景（BFS 最短路径）

use std::{collections::HashMap, thread, time::Duration};

use anyhow::{Result, bail};
use tracing::{debug, info, warn};

use super::{
    model::{Scene, SceneId},
    route_executor::{RouteExecutionOutcome, RouteExecutor},
    route_planner::RoutePlanner,
    scene_detector::SceneDetector,
};
use crate::session::Session;

/// 场景管理器：负责场景检测和导航。
///
/// 注册所有已知场景后，可以自动检测当前场景并从任意受支持场景导航到目标场景。
pub struct SceneManager {
    scene_detector: SceneDetector,
    route_planner: RoutePlanner,
}

impl SceneManager {
    /// 注册所有场景，并根据其跳转关系构建不可变的路由规划器。
    ///
    /// 场景按注册顺序排列识别优先级——应先注册更具体的场景（如"档案详情页面"），
    /// 再注册更笼统的场景（如"大世界"、"未知"）。
    ///
    /// # Panics
    ///
    /// 同一个 `SceneId` 变体只能注册一个场景识别器。带负载的变体应由同一个
    /// 识别器返回具体 ID，例如 `档案库子界面`。
    pub fn new(scenes: Vec<Box<dyn Scene>>) -> Self {
        let mut navigation_graph = HashMap::new();

        for scene in &scenes {
            let id = scene.id();
            navigation_graph.insert(
                id,
                scene
                    .transitions()
                    .iter()
                    .map(|transition| transition.target)
                    .collect(),
            );
        }

        Self {
            scene_detector: SceneDetector::new(scenes),
            route_planner: RoutePlanner::new(navigation_graph),
        }
    }

    // ========== 场景检测 ==========

    /// 检测当前处于哪个场景。
    ///
    /// 按注册顺序遍历所有场景的 `try_recognize()`，返回第一个成功识别的场景 ID。
    /// 如果所有场景都无法识别，返回 `SceneId::未知`。
    pub fn detect_current_scene(&self, session: &mut Session) -> Result<SceneId> {
        self.scene_detector.detect_current_scene(session)
    }

    /// 持续检测直到场景变为指定场景或超时。
    ///
    /// # 参数
    /// - `expected`: 期望的目标场景
    /// - `session`: 会话上下文
    /// - `max_retries`: 最大重试次数
    ///
    /// 每次重试之间会短暂等待（约 200ms），给游戏界面切换留出时间。
    pub fn wait_for_scene(
        &self,
        expected: SceneId,
        session: &mut Session,
        max_retries: u32,
    ) -> Result<bool> {
        for i in 0..max_retries {
            let current = self.detect_current_scene(session)?;
            if current == expected {
                debug!("等待场景: 已到达目标 {:?} (第 {i} 次检测)", expected);
                return Ok(true);
            }
            debug!(
                "等待场景: 当前 {:?}, 期望 {:?} (第{i}/{max_retries})",
                current, expected
            );
            thread::sleep(Duration::from_millis(200));
        }
        Ok(false)
    }

    /// 检查当前是否为指定场景，不执行导航。
    ///
    /// 仅调用期望场景对应的识别器，避免遍历所有已注册场景。
    /// 对于档案库子界面，识别结果必须与期望的具体子界面完全一致。
    pub fn require_scene(&self, expected: SceneId, session: &mut Session) -> Result<()> {
        if self.scene_detector.recognizes_scene(expected, session)? {
            Ok(())
        } else {
            bail!("当前场景不符合预期: {:?}", expected)
        }
    }

    // ========== 场景导航 ==========

    /// 从当前场景导航到目标场景（BFS 最短路径）。
    ///
    /// 自动计算并执行最短跳转路径。
    ///
    /// # 参数
    /// - `target`: 目标场景
    /// - `session`: 会话上下文
    ///
    /// # 工作流程
    /// 1. 检测当前场景
    /// 2. 如果已在目标场景，直接返回
    /// 3. BFS 搜索最短路径
    /// 4. 依次执行路径上的跳转动作，每步执行后重新检测场景。
    ///    如果某一步未到达预期场景，则重试该步骤；连续失败 3 次则从当前场景重新规划路径。
    pub fn navigate_to(&self, target: SceneId, session: &mut Session) -> Result<()> {
        const MAX_REPLANS: u32 = 5;

        let mut current = self.detect_current_scene(session)?;
        if current == target {
            debug!("导航: 已在目标场景 {:?}，无需跳转", target);
            return Ok(());
        }
        if current == SceneId::未知 {
            bail!("当前场景无法识别，无法导航到 {:?}", target);
        }

        let mut replan_count = 0;
        loop {
            let route = self.route_planner.find_route(current, target)?;
            if replan_count == 0 {
                info!(
                    "导航: {:?} → {:?}, 路径: {:?}",
                    current, target, route.steps
                );
            } else {
                info!("重新规划路径: {:?}", route.steps);
            }

            let outcome = RouteExecutor::new(&self.scene_detector, session, route).run()?;
            current = outcome.observed_scene();
            if outcome.observed_scene() == target {
                info!("导航完成: 已到达 {:?}", target);
                return Ok(());
            }

            if replan_count == MAX_REPLANS {
                bail!(
                    "导航失败: 已重新规划 {MAX_REPLANS} 次，仍无法到达 {:?}",
                    target
                );
            }
            replan_count += 1;
            match outcome {
                RouteExecutionOutcome::RouteFinished { .. } => warn!(
                    "路由执行完成但未到达预期场景 {:?}，当前为 {:?}，重新规划 ({}/{})",
                    target, current, replan_count, MAX_REPLANS
                ),
                RouteExecutionOutcome::NeedsReplan { .. } => warn!(
                    "路由执行中断，当前场景 {:?}，重新规划到 {:?} ({}/{})",
                    current, target, replan_count, MAX_REPLANS
                ),
            }
        }
    }

    /// 确保当前处于目标场景，如果不在则自动导航过去。
    ///
    /// 先仅调用目标场景的识别器；识别不匹配时再执行完整导航。
    pub fn ensure_scene(&self, target: SceneId, session: &mut Session) -> Result<()> {
        if self.scene_detector.recognizes_scene(target, session)? {
            return Ok(());
        }
        self.navigate_to(target, session)
    }
}
