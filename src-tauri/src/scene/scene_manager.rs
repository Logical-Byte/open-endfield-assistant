//! 场景管理器。
//!
//! 提供：
//! 1. 场景检测：自动判断当前处于哪个游戏界面
//! 2. 场景导航：从任意受支持场景自动跳转到目标场景（BFS 最短路径）

use std::{
    collections::{HashMap, VecDeque},
    mem::{Discriminant, discriminant},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tracing::{debug, info, warn};

use super::{Scene, SceneId, 档案库SubSceneId};
use crate::session::Session;

/// SceneManager 内部使用的规范化场景 ID。
///
/// 因为 `SceneId::档案库子界面(kind)` 有多个变体（6 个子分类），
/// 但导航图中只注册了一个（`Scene档案库子界面.id()` 返回的那个）。
/// 此函数将所有 `档案库子界面(*)` 统一归一化为 `档案库子界面(音像存档_多媒体)`，
/// 用于导航图的查找。
fn normalize_scene_id(id: SceneId) -> SceneId {
    match id {
        SceneId::档案库子界面(_) => {
            SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体)
        }
        other => other,
    }
}

/// 场景管理器：负责场景检测和导航。
///
/// 注册所有已知场景后，可以自动检测当前场景并从任意受支持场景导航到目标场景。
pub struct SceneManager {
    /// 所有已注册的场景（按优先级排序，越具体的场景越靠前）
    scenes: Vec<Box<dyn Scene>>,
    /// 场景 ID 变体 → 索引的快速查找表
    scene_index: HashMap<Discriminant<SceneId>, usize>,
    /// 导航图：scene_id → 所有可达的 (target_id, transition_index)
    navigation_graph: HashMap<SceneId, Vec<(SceneId, usize)>>,
}

impl SceneManager {
    /// 创建空的 SceneManager，随后调用 `register()` 注册场景。
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            scene_index: HashMap::new(),
            navigation_graph: HashMap::new(),
        }
    }

    /// 注册一个场景。
    ///
    /// 场景按注册顺序排列识别优先级——应先注册更具体的场景（如"档案详情页面"），
    /// 再注册更笼统的场景（如"大世界"、"未知"）。
    ///
    /// # Panics
    ///
    /// 同一个 `SceneId` 变体只能注册一个场景识别器。带负载的变体应由同一个
    /// 识别器返回具体 ID，例如 `档案库子界面`。
    pub fn register(&mut self, scene: Box<dyn Scene>) {
        let id = scene.id();
        let idx = self.scenes.len();
        let key = discriminant(&id);
        assert!(
            !self.scene_index.contains_key(&key),
            "同一 SceneId 变体重复注册: {:?}",
            id
        );
        self.scene_index.insert(key, idx);
        self.scenes.push(scene);
    }

    /// 完成注册后调用，构建导航图。
    ///
    /// 必须在所有场景注册完毕后调用一次。
    pub fn build_navigation_graph(&mut self) {
        self.navigation_graph.clear();
        for scene in self.scenes.iter() {
            let from_id = scene.id();
            let edges: Vec<(SceneId, usize)> = scene
                .transitions()
                .iter()
                .enumerate()
                .map(|(ti, t)| (t.target, ti))
                .collect();
            self.navigation_graph.insert(from_id, edges);
        }
    }

    // ========== 场景检测 ==========

    /// 检测当前处于哪个场景。
    ///
    /// 按注册顺序遍历所有场景的 `try_recognize()`，返回第一个成功识别的场景 ID。
    /// 如果所有场景都无法识别，返回 `SceneId::未知`。
    pub fn detect_current_scene(&self, session: &mut Session) -> Result<SceneId> {
        let _screenshot = session.screencap_for_recognition()?;
        // 截图后先记下，后续 try_recognize 不需要重复截图
        // 注意：try_recognize 可能会内部再截图（比如子界面颜色判断），
        // 但大多数场景只需要一张截图即可判断

        for scene in &self.scenes {
            // 识别出错（如模板缺失、模板尺寸与 ROI 不匹配）属于严重问题，
            // 用 ? 直接让任务失败，而不是继续识别最后误报「未知场景」掩盖真实原因
            let id = scene
                .try_recognize(session)
                .with_context(|| format!("场景识别出错 ({})", scene.name()))?;
            if let Some(id) = id {
                debug!("场景检测: 当前处于 {:?}", id);
                return Ok(id);
            }
        }

        warn!("场景检测: 当前场景无法识别，返回「未知」");
        Ok(SceneId::未知)
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
        if self.recognizes_scene(expected, session)? {
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
        // 1. 检测当前场景
        let current = self.detect_current_scene(session)?;
        if current == target {
            debug!("导航: 已在目标场景 {:?}，无需跳转", target);
            return Ok(());
        }

        if current == SceneId::未知 {
            bail!("当前场景无法识别，无法导航到 {:?}", target);
        }

        // 2. BFS 寻找最短路径
        let mut path = self.find_path(current, target)?;
        info!("导航: {:?} → {:?}, 路径: {:?}", current, target, path);

        // 3. 依次执行路径上的跳转，每步失败时重试
        const MAX_RETRIES_PER_STEP: u32 = 3;
        const MAX_REPLANS: u32 = 5;
        let mut i = 0usize;
        let mut replan_count = 0u32;
        while i < path.len() {
            let (from, to) = path[i];
            debug!("导航步骤 {}/{}: {:?} → {:?}", i + 1, path.len(), from, to);

            // 执行跳转动作
            self.execute_single_step(from, to, session)?;

            // 等待并验证结果
            let mut step_ok = false;
            for retry in 0..MAX_RETRIES_PER_STEP {
                thread::sleep(Duration::from_millis(500));
                let after = self.detect_current_scene(session)?;
                // 从档案详情返回时会回到进入详情前的任意子界面（不确定），
                // 因此只要到达任意档案库子界面即视为命中；其余步骤严格匹配目标场景。
                let arrived = if from == SceneId::档案详情页面 {
                    matches!(after, SceneId::档案库子界面(_))
                } else {
                    after == to
                };
                if arrived {
                    debug!("导航步骤 {i} 成功，已到达 {:?}", to);
                    step_ok = true;
                    break;
                }
                if retry < MAX_RETRIES_PER_STEP - 1 {
                    warn!(
                        "导航步骤 {i}: 预期到达 {:?}，实际检测到 {:?}，重试 ({}/{})",
                        to,
                        after,
                        retry + 1,
                        MAX_RETRIES_PER_STEP
                    );
                    // 重新执行跳转动作（比如再按一次 ESC）
                    self.execute_single_step(from, to, session)?;
                }
            }

            if step_ok {
                i += 1; // 成功，前进到下一步
            } else {
                // 重试耗尽仍未到达，从当前场景重新规划
                replan_count += 1;
                if replan_count > MAX_REPLANS {
                    bail!(
                        "导航失败: 已重新规划 {MAX_REPLANS} 次，仍无法到达 {:?}",
                        target
                    );
                }
                let current = self.detect_current_scene(session)?;
                warn!(
                    "导航步骤 {i} 失败，当前场景 {:?}，重新规划路径到 {:?}",
                    current, target
                );
                let new_path = self.find_path(current, target)?;
                info!("重新规划路径: {:?}", new_path);
                // 用新路径替换剩余步骤，重置索引
                // 将新路径追加到当前位置之后
                path.truncate(i);
                path.extend(new_path);
                // 不递增 i，从当前步骤重新开始
            }
        }

        // 最终验证
        let final_scene = self.detect_current_scene(session)?;
        if final_scene == target {
            info!("导航完成: 已到达 {:?}", target);
            Ok(())
        } else {
            // 再做一次最终尝试
            warn!(
                "导航未到达预期场景 {:?}，当前为 {:?}，重试",
                target, final_scene
            );
            self.navigate_to(target, session)
        }
    }

    /// 确保当前处于目标场景，如果不在则自动导航过去。
    ///
    /// 先仅调用目标场景的识别器；识别不匹配时再执行完整导航。
    pub fn ensure_scene(&self, target: SceneId, session: &mut Session) -> Result<()> {
        if self.recognizes_scene(target, session)? {
            return Ok(());
        }
        self.navigate_to(target, session)
    }

    // ========== 内部方法 ==========

    /// 根据场景 ID 变体查找已注册的场景识别器。
    fn get_registered_scene(&self, hint: SceneId) -> Option<&dyn Scene> {
        let idx = self.scene_index.get(&discriminant(&hint))?;
        Some(self.scenes[*idx].as_ref())
    }

    /// 仅使用期望场景的识别器检查当前场景。
    fn recognizes_scene(&self, expected: SceneId, session: &mut Session) -> Result<bool> {
        if expected == SceneId::未知 {
            bail!("未知场景不能作为预期场景");
        }

        let scene = self
            .get_registered_scene(expected)
            .ok_or_else(|| anyhow::anyhow!("未注册的场景: {:?}", expected))?;
        let recognized = scene
            .try_recognize(session)
            .with_context(|| format!("场景识别出错 ({})", scene.name()))?;
        Ok(recognized == Some(expected))
    }

    /// BFS 寻找从 `from` 到 `to` 的最短路径。
    ///
    /// 返回路径上每一步的 (from_scene, to_scene) 对。
    fn find_path(&self, from: SceneId, to: SceneId) -> Result<Vec<(SceneId, SceneId)>> {
        if from == to {
            return Ok(Vec::new());
        }

        // BFS，对 档案库子界面 进行归一化以匹配导航图中的注册项
        let lookup_from = normalize_scene_id(from);

        let mut queue = VecDeque::new();
        let mut visited = HashMap::new(); // scene_id → (prev_scene_id, transition_index)
        visited.insert(lookup_from, (lookup_from, 0usize)); // 起点标记
        queue.push_back(lookup_from);

        while let Some(current) = queue.pop_front() {
            if let Some(edges) = self.navigation_graph.get(&current) {
                for &(next_id, _ti) in edges {
                    if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(next_id) {
                        e.insert((current, 0));
                        if next_id == to || normalize_scene_id(next_id) == normalize_scene_id(to) {
                            // 找到目标，回溯路径
                            return Ok(self.reconstruct_path(lookup_from, next_id, &visited));
                        }
                        queue.push_back(next_id);
                    }
                }
            }
        }

        bail!("导航失败: 无法从 {:?} 到达 {:?}", from, to)
    }

    /// 从 BFS visited 表中回溯路径。
    fn reconstruct_path(
        &self,
        from: SceneId,
        to: SceneId,
        visited: &HashMap<SceneId, (SceneId, usize)>,
    ) -> Vec<(SceneId, SceneId)> {
        let mut path = Vec::new();
        let mut current = to;
        while current != from {
            let &(prev, _) = visited.get(&current).unwrap();
            path.push((prev, current));
            current = prev;
        }
        path.reverse();
        path
    }

    /// 执行单步跳转：从 `from` 场景跳转到 `to` 场景。
    fn execute_single_step(&self, from: SceneId, to: SceneId, session: &mut Session) -> Result<()> {
        let scene = self
            .get_registered_scene(from)
            .ok_or_else(|| anyhow::anyhow!("未注册的场景: {:?}", from))?;
        scene.execute_transition(to, session)
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}
