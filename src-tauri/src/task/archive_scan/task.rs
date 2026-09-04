//! 档案库扫描任务定义。

use std::thread;
use std::time::Duration;

use anyhow::Result;
use tracing::info;

use crate::automation::{AutomateExecutor, Point720p};
use crate::{
    scene::{
        AutomateAction, SceneId,
        archive::{ROI_中枢档案按钮, ROI_见闻辑录按钮, ROI_音像存档按钮},
        scene_manager::SceneManager,
        档案库SubSceneId,
    },
    session::Session,
    task::Task,
};

use super::correction::{CorrectionOverride, DEFAULT_CORRECTION_OVERRIDES};
use super::plan::{SCAN_PLAN, ScanStep};
use super::result::ScanReporter;
use super::scan_loop::scan_current_sub_scene;
use crate::data::ArchiveTitleIndex;

/// 扫描档案库任务：扫描全部 6 个子分类的档案。
///
/// 扫描结果上报器与档案标题索引在构造时由调用方（[`crate::controller::Controller`]）注入，
/// 使 `Task` trait 保持通用、不耦合档案上报。
pub struct ArchiveScanTask<'a> {
    reporter: ScanReporter,
    archive_titles: &'a ArchiveTitleIndex,
    correction_overrides: Option<&'a [CorrectionOverride<'a>]>,
}

impl<'a> ArchiveScanTask<'a> {
    /// 创建任务。
    pub fn new(reporter: ScanReporter, archive_titles: &'a ArchiveTitleIndex) -> Self {
        Self {
            reporter,
            archive_titles,
            correction_overrides: Some(DEFAULT_CORRECTION_OVERRIDES),
        }
    }
}

impl Task for ArchiveScanTask<'_> {
    fn name(&self) -> &str {
        "扫描档案库"
    }

    fn precondition_scene(&self) -> SceneId {
        SceneId::档案库主界面
    }

    fn run(&self, session: &mut Session, scene_manager: &SceneManager) -> Result<()> {
        // Step 1: 遍历所有子分类
        for (step_idx, step) in SCAN_PLAN.iter().enumerate() {
            info!(
                "===== 扫描子分类 {}/{}: {:?} =====",
                step_idx + 1,
                SCAN_PLAN.len(),
                step.first_sub_scene
            );

            // 1a. 从档案库主界面点击入口按钮进入子分类
            self.enter_sub_scene_from_main(session, scene_manager, step)?;

            // 1b. 遍历该分类下的所有子界面
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
                    self.archive_titles,
                    self.correction_overrides,
                    &self.reporter,
                )?;
                info!("完成扫描 {:?}", sub_scene);
            }

            // 1c. 返回档案库主界面（准备进入下一个子分类）
            info!("返回档案库主界面...");
            scene_manager.navigate_to(SceneId::档案库主界面, session)?;
        }

        info!("全部 6 个子分类扫描完毕！");
        Ok(())
    }
}

impl ArchiveScanTask<'_> {
    /// 从档案库主界面点击入口按钮进入子分类。
    ///
    /// 调用时应已处于档案库主界面。
    fn enter_sub_scene_from_main(
        &self,
        session: &mut Session,
        scene_manager: &SceneManager,
        step: &ScanStep,
    ) -> Result<()> {
        scene_manager.require_scene(SceneId::档案库主界面, session)?;

        // 构造并执行入口按钮的模板动作
        let roi = match step.first_sub_scene {
            档案库SubSceneId::音像存档_多媒体 => ROI_音像存档按钮,
            档案库SubSceneId::见闻辑录_纸质记录 => ROI_见闻辑录按钮,
            档案库SubSceneId::中枢档案_中枢档案 => ROI_中枢档案按钮,
            _ => ROI_音像存档按钮, // fallback
        };

        let action = AutomateAction::FindAndClickTemplate(crate::automation::TemplateTarget {
            template_name: step.entry_template,
            roi,
            threshold: 0.75,
        });
        session.automate_context().execute(&action)??;

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
    /// 点击逻辑与 tab 坐标单点维护在档案库场景定义中。
    fn switch_sub_tab(&self, session: &mut Session, tab_index: usize) -> Result<()> {
        let roi = crate::scene::archive::TAB_ROIS
            .get(tab_index)
            .ok_or_else(|| anyhow::anyhow!("无效的 tab 索引: {tab_index}"))?;
        let center = roi.center();
        let action = AutomateAction::ClickAt(Point720p {
            x: center.x,
            y: center.y,
        });
        session.automate_context().execute(&action)??;

        // 等待界面切换
        thread::sleep(Duration::from_millis(800));

        Ok(())
    }
}
