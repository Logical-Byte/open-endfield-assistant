//! 档案库扫描任务定义。

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use tracing::info;

use crate::{
    scene::{SceneAction, SceneId, scene_manager::SceneManager, 档案库SubSceneId},
    session::Session,
    task::Task,
    utils::region::Region2D,
};

use super::correction::CorrectionIndex;
use super::plan::{SCAN_PLAN, ScanStep};
use super::result::ScanReporter;
use super::scan_loop::scan_current_sub_scene;

/// 扫描档案库任务：扫描全部 6 个子分类的档案。
///
/// 扫描结果上报器与纠错索引在构造时由调用方（[`crate::controller::Controller`]）注入，
/// 使 `Task` trait 保持通用、不耦合档案上报。
pub struct ArchiveScanTask {
    reporter: ScanReporter,
    correction: Arc<CorrectionIndex>,
}

impl ArchiveScanTask {
    /// 创建任务。
    pub fn new(reporter: ScanReporter, correction: Arc<CorrectionIndex>) -> Self {
        Self {
            reporter,
            correction,
        }
    }
}

impl Task for ArchiveScanTask {
    fn name(&self) -> &str {
        "扫描档案库"
    }

    fn supported_entry_scenes(&self) -> &[SceneId] {
        // 任务支持从任意档案库相关界面、协议终端或大世界开始：
        // - 大世界 → 自动导航到 协议终端 → 档案库主界面
        // - 协议终端 → 自动导航到 档案库主界面
        // - 档案库主界面 → 无需导航，直接开始扫描
        // - 档案库子界面（任意分类）→ 自动导航到 档案库主界面
        // - 档案详情页面 → 自动导航到 档案库子界面 → 档案库主界面
        static ENTRIES: std::sync::LazyLock<Vec<SceneId>> = std::sync::LazyLock::new(|| {
            use 档案库SubSceneId::*;
            vec![
                SceneId::大世界,
                SceneId::协议终端,
                SceneId::档案库主界面,
                SceneId::档案库子界面(音像存档_多媒体),
                SceneId::档案库子界面(见闻辑录_纸质记录),
                SceneId::档案库子界面(见闻辑录_电子档案),
                SceneId::档案库子界面(见闻辑录_藏品),
                SceneId::档案库子界面(中枢档案_中枢档案),
                SceneId::档案库子界面(中枢档案_调查报告),
                SceneId::档案详情页面,
            ]
        });
        &ENTRIES
    }

    fn run(&self, session: &mut Session, scene_manager: &SceneManager) -> Result<()> {
        // Step 1: 确保在档案库主界面
        info!("导航到档案库主界面...");
        scene_manager.ensure_scene(SceneId::档案库主界面, session)?;

        // Step 2: 遍历所有子分类
        for (step_idx, step) in SCAN_PLAN.iter().enumerate() {
            info!(
                "===== 扫描子分类 {}/{}: {:?} =====",
                step_idx + 1,
                SCAN_PLAN.len(),
                step.first_sub_scene
            );

            // 2a. 从档案库主界面点击入口按钮进入子分类
            self.enter_sub_scene_from_main(session, scene_manager, step)?;

            // 2b. 遍历该分类下的所有子界面
            for (sub_idx, &sub_scene) in step.sub_scenes.iter().enumerate() {
                if sub_idx > 0 {
                    // 不是第一个子界面，需要点击侧边栏 tab 切换（索引即 tab 序号）
                    let tab_index = sub_idx;
                    info!("切换到子界面: {:?} (点击 tab #{})", sub_scene, tab_index);
                    self.switch_sub_tab(session, tab_index)?;
                }

                // 等待界面稳定
                thread::sleep(Duration::from_millis(500));

                // 扫描该子界面中的所有档案
                info!("开始扫描 {:?} 中的档案...", sub_scene);
                scan_current_sub_scene(
                    session,
                    scene_manager,
                    sub_scene,
                    &self.correction,
                    &self.reporter,
                )?;
                info!("完成扫描 {:?}", sub_scene);
            }

            // 2c. 返回档案库主界面（准备进入下一个子分类）
            info!("返回档案库主界面...");
            scene_manager.navigate_to(SceneId::档案库主界面, session)?;
        }

        info!("全部 6 个子分类扫描完毕！");
        Ok(())
    }
}

impl ArchiveScanTask {
    /// 从档案库主界面点击入口按钮进入子分类。
    fn enter_sub_scene_from_main(
        &self,
        session: &mut Session,
        scene_manager: &SceneManager,
        step: &ScanStep,
    ) -> Result<()> {
        // 确保在主界面
        scene_manager.ensure_scene(SceneId::档案库主界面, session)?;

        // 截图并搜索入口按钮
        let screenshot = session.screencap_for_recognition()?;
        let roi = match step.first_sub_scene {
            档案库SubSceneId::音像存档_多媒体 => Region2D::from_ltrb(692, 371, 959, 601),
            档案库SubSceneId::见闻辑录_纸质记录 => {
                Region2D::from_ltrb(957, 135, 1221, 371)
            }
            档案库SubSceneId::中枢档案_中枢档案 => {
                Region2D::from_ltrb(958, 369, 1220, 601)
            }
            _ => Region2D::from_ltrb(692, 371, 959, 601), // fallback
        };

        let found = session.find_and_click_template(&screenshot, step.entry_template, roi, 0.75)?;
        if !found {
            anyhow::bail!("在档案库主界面未找到入口按钮: {}", step.entry_template);
        }

        // 等待跳转完成
        thread::sleep(Duration::from_millis(800));

        // 验证是否进入了目标子界面
        let target_id = SceneId::档案库子界面(step.first_sub_scene);
        let arrived = scene_manager.wait_for_scene(target_id, session, 10)?;
        if !arrived {
            anyhow::bail!("未能进入子界面 {:?}", step.first_sub_scene);
        }

        Ok(())
    }

    /// 在同一分类内切换子界面（点击侧边栏 tab）。
    ///
    /// 点击逻辑与 tab 坐标单点维护在 [`SceneAction::ClickSubTab`]（scene_action.rs），
    /// 此处只构造动作并执行，避免坐标重复。
    fn switch_sub_tab(&self, session: &mut Session, tab_index: usize) -> Result<()> {
        let screenshot = session.screencap_for_recognition()?;
        SceneAction::ClickSubTab {
            roi_index: tab_index,
        }
        .execute(session, &screenshot)?;

        // 等待界面切换
        thread::sleep(Duration::from_millis(800));

        Ok(())
    }
}
