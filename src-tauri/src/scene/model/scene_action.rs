//! 场景跳转动作定义与执行逻辑。

use anyhow::Result;
use image::RgbaImage;

use crate::{
    session::Session,
    utils::region::{Region2D, ltwh},
};

/// 档案库侧边栏从上到下三个 tab 的颜色检测与点击区域（720p 基准 LTWH）。
pub(crate) const TAB_ROIS: [Region2D<u32>; 3] = [
    ltwh!(180, 120, 60, 36),
    ltwh!(180, 184, 60, 36),
    ltwh!(180, 248, 60, 36),
];

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
                let roi = TAB_ROIS
                    .get(*roi_index)
                    .ok_or_else(|| anyhow::anyhow!("无效的 ROI 索引: {roi_index}"))?;
                let center = roi.center();
                session.click_at_720p(center.x, center.y)?;
            }
            SceneAction::PressKey { vk_code } => {
                session.press_key(*vk_code)?;
            }
        }
        Ok(())
    }
}
