//! 场景系统模块。
//!
//! 借鉴 MaaFramework 的 "识别 → 动作 → next" 节点模型，每个游戏界面实现
//! [`Scene`] trait，负责自我识别和跳转。

pub mod scene_manager;

use anyhow::Result;
use image::RgbaImage;

use crate::{session::Session, utils::region::Region2D};

// ============================================================================
// SceneId — 场景唯一标识符
// ============================================================================

/// 场景的唯一标识符，对应游戏中的每一个 UI 界面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneId {
    /// 大世界界面
    大世界,
    /// 协议终端界面
    协议终端,
    /// 档案库主界面
    档案库主界面,
    /// 档案库子界面（带具体分类）
    档案库子界面(SubSceneKind),
    /// 档案详情页面（查看单份档案内容）
    档案详情页面,
    /// 未知界面（无法识别的界面）
    未知,
}

/// 档案库子界面的具体分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubSceneKind {
    /// 音像存档 - 多媒体（只有这一个子界面）
    音像存档_多媒体,
    /// 见闻辑录 - 纸质记录
    见闻辑录_纸质记录,
    /// 见闻辑录 - 电子档案
    见闻辑录_电子档案,
    /// 见闻辑录 - 藏品
    见闻辑录_藏品,
    /// 中枢档案 - 中枢档案
    中枢档案_中枢档案,
    /// 中枢档案 - 调查报告
    中枢档案_调查报告,
}

// ============================================================================
// SceneAction — 跳转动作
// ============================================================================

/// 从一个场景跳转到另一个场景的具体动作。
pub enum SceneAction {
    /// 在指定 ROI 内搜索模板按钮，找到后点击
    FindAndClickTemplate {
        /// 模板名称（如 `"情报档案库/音像存档.png"`）
        template_name: &'static str,
        /// 搜索区域（720p 基准 ltrb）
        roi: Region2D<u32>,
        /// 匹配阈值
        threshold: f32,
    },
    /// 点击固定坐标（720p 基准）
    ClickAt { x: u32, y: u32 },
    /// 点击侧边栏第 N 个 tab（通过颜色 ROI 切换子界面）
    /// `roi_index`: 0-based，对应 ltwh(180, 120+64*i, 60, 36)
    ClickSubTab { roi_index: usize },
    /// 按下键盘按键（虚拟键码）
    PressKey { vk_code: i32 },
}

// ============================================================================
// Scene — 场景 trait
// ============================================================================

/// 场景 trait：每个游戏界面实现此 trait。
///
/// 借鉴 MaaFramework Pipeline 节点的设计，每个场景负责：
/// 1. 识别自身（`try_recognize`）
/// 2. 定义可跳转的目标场景（`transitions`）
pub trait Scene {
    /// 返回此场景的唯一标识符。
    fn id(&self) -> SceneId;

    /// 返回场景名称（用于日志）。
    fn name(&self) -> &'static str;

    /// 尝试识别当前截图是否为此场景。
    ///
    /// # 参数
    /// - `session`: 会话上下文（从中获取截图等）
    ///
    /// # 返回
    /// - `Ok(Some(scene_id))`: 识别成功，返回确切的场景 ID（子界面可能需要返回更具体的 ID）
    /// - `Ok(None)`: 不是此场景
    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>>;

    /// 返回从此场景可以跳转到的目标场景及跳转方式。
    ///
    /// 注意：这是所有可能的跳转，实际执行时会按顺序尝试，第一个成功即停止。
    fn transitions(&self) -> &[SceneTransition];

    /// 执行跳转动作以到达目标场景。
    ///
    /// 默认实现：从 `transitions()` 中找到去往 `target` 的动作并执行。
    fn execute_transition(&self, target: SceneId, session: &mut Session) -> Result<()> {
        let screenshot = session.screencap_for_recognition()?;
        for transition in self.transitions() {
            if transition.target == target {
                transition.action.execute(session, &screenshot)?;
                return Ok(());
            }
        }
        anyhow::bail!("没有从 {:?} 到 {:?} 的跳转", self.id(), target);
    }
}

/// 场景跳转描述：从当前场景到目标场景以及执行方式。
pub struct SceneTransition {
    /// 目标场景 ID
    pub target: SceneId,
    /// 跳转动作
    pub action: SceneAction,
}

// ============================================================================
// SceneAction 执行逻辑
// ============================================================================

impl SceneAction {
    /// 执行跳转动作。
    ///
    /// # 参数
    /// - `session`: 会话上下文
    /// - `screenshot`: 当前 720p 截图（用于模板匹配）
    pub fn execute(&self, session: &mut Session, screenshot: &RgbaImage) -> Result<()> {
        match self {
            SceneAction::FindAndClickTemplate {
                template_name,
                roi,
                threshold,
            } => {
                let found =
                    session.find_and_click_template(screenshot, template_name, *roi, *threshold)?;
                if !found {
                    anyhow::bail!("未找到模板: {template_name} 在 ROI {roi:?}");
                }
            }
            SceneAction::ClickAt { x, y } => {
                session.click_at_720p(*x, *y)?;
            }
            SceneAction::ClickSubTab { roi_index } => {
                // 颜色 ROI 的 ltwh 坐标：从上到下分别是 (180, 120, 60, 36)、
                // (180, 184, 60, 36)、(180, 248, 60, 36)
                const TAB_ROIS: [(u32, u32, u32, u32); 3] =
                    [(180, 120, 60, 36), (180, 184, 60, 36), (180, 248, 60, 36)];
                let (x, y, w, h) = TAB_ROIS
                    .get(*roi_index)
                    .ok_or_else(|| anyhow::anyhow!("无效的 ROI 索引: {roi_index}"))?;
                // 点击 ROI 中心
                let cx = x + w / 2;
                let cy = y + h / 2;
                session.click_at_720p(cx, cy)?;
            }
            SceneAction::PressKey { vk_code } => {
                session.press_key(*vk_code)?;
            }
        }
        Ok(())
    }
}
