//! 扫描档案库任务。
//!
//! 从任意受支持的界面出发，导航到档案库主界面，遍历全部 6 个子分类，
//! 扫描每个子分类中的所有档案，OCR 识别档案标题并记录 SUCCESS 日志。

mod scan_loop;
pub mod scenes;

use anyhow::Result;
use tracing::info;

use crate::{
    scene::{SceneId, SubSceneKind, scene_manager::SceneManager},
    session::Session,
    task::Task,
};

use self::scan_loop::scan_current_sub_scene;

// ============================================================================
// 扫描计划：定义 6 个子分类的遍历顺序
// ============================================================================

/// 一个扫描步骤：从档案库主界面点击哪个按钮进入哪个子分类，该分类下有哪些子界面需要扫描。
struct ScanStep {
    /// 从档案库主界面点击此模板进入该子分类
    entry_template: &'static str,
    /// 进入后的初始子界面
    first_sub_scene: SubSceneKind,
    /// 该分类下需要扫描的所有子界面（按 tab 顺序排列）
    /// 音像存档: [多媒体]
    /// 见闻辑录: [纸质记录, 电子档案, 藏品]
    /// 中枢档案: [中枢档案, 调查报告]
    sub_scenes: &'static [SubSceneKind],
}

/// 6 个子分类的完整扫描计划。
const SCAN_PLAN: &[ScanStep] = &[
    ScanStep {
        // 音像存档 — 只有 1 个子界面
        entry_template: "情报档案库/音像存档.png",
        first_sub_scene: SubSceneKind::音像存档_多媒体,
        sub_scenes: &[SubSceneKind::音像存档_多媒体],
    },
    ScanStep {
        // 见闻辑录 — 有 3 个子界面（纸质记录、电子档案、藏品）
        entry_template: "情报档案库/见闻辑录.png",
        first_sub_scene: SubSceneKind::见闻辑录_纸质记录,
        sub_scenes: &[
            SubSceneKind::见闻辑录_纸质记录,
            SubSceneKind::见闻辑录_电子档案,
            SubSceneKind::见闻辑录_藏品,
        ],
    },
    ScanStep {
        // 中枢档案 — 有 2 个子界面（中枢档案、调查报告）
        entry_template: "情报档案库/中枢档案.png",
        first_sub_scene: SubSceneKind::中枢档案_中枢档案,
        sub_scenes: &[
            SubSceneKind::中枢档案_中枢档案,
            SubSceneKind::中枢档案_调查报告,
        ],
    },
];

// ============================================================================
// ArchiveScanTask
// ============================================================================

/// 扫描档案库任务：扫描所有档案的子分类和档案详情。
pub struct ArchiveScanTask;

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
            use SubSceneKind::*;
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
                    // 不是第一个子界面，需要点击侧边栏 tab 切换
                    // tab 索引: 0=第1个, 1=第2个, 2=第3个
                    let tab_index = sub_idx; // 子界面的索引就是 tab 索引
                    info!("切换到子界面: {:?} (点击 tab #{})", sub_scene, tab_index);
                    self.switch_sub_tab(session, scene_manager, tab_index)?;
                }

                // 等待界面稳定
                std::thread::sleep(std::time::Duration::from_millis(500));

                // 扫描该子界面中的所有档案
                info!("开始扫描 {:?} 中的档案...", sub_scene);
                scan_current_sub_scene(session, scene_manager)?;
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
            SubSceneKind::音像存档_多媒体 => {
                crate::utils::region::Region2D::from_ltrb(692, 371, 959, 601)
            }
            SubSceneKind::见闻辑录_纸质记录 => {
                crate::utils::region::Region2D::from_ltrb(957, 135, 1221, 371)
            }
            SubSceneKind::中枢档案_中枢档案 => {
                crate::utils::region::Region2D::from_ltrb(958, 369, 1220, 601)
            }
            _ => crate::utils::region::Region2D::from_ltrb(692, 371, 959, 601), // fallback
        };

        let found = session.find_and_click_template(&screenshot, step.entry_template, roi, 0.75)?;
        if !found {
            anyhow::bail!("在档案库主界面未找到入口按钮: {}", step.entry_template);
        }

        // 等待跳转完成
        std::thread::sleep(std::time::Duration::from_millis(800));

        // 验证是否进入了目标子界面
        let target_id = SceneId::档案库子界面(step.first_sub_scene);
        let arrived = scene_manager.wait_for_scene(target_id, session, 10)?;
        if !arrived {
            anyhow::bail!("未能进入子界面 {:?}", step.first_sub_scene);
        }

        Ok(())
    }

    /// 在同一分类内切换子界面（点击侧边栏 tab）。
    fn switch_sub_tab(
        &self,
        session: &mut Session,
        _scene_manager: &SceneManager,
        tab_index: usize,
    ) -> Result<()> {
        // 颜色 ROI 的 ltwh 坐标
        const TAB_ROIS: [(u32, u32, u32, u32); 3] = [
            (180, 120, 60, 36), // tab 0
            (180, 184, 60, 36), // tab 1
            (180, 248, 60, 36), // tab 2
        ];

        let (x, y, w, h) = TAB_ROIS
            .get(tab_index)
            .ok_or_else(|| anyhow::anyhow!("无效的 tab 索引: {tab_index}"))?;
        let cx = x + w / 2;
        let cy = y + h / 2;
        session.click_at_720p(cx, cy)?;

        // 等待界面切换
        std::thread::sleep(std::time::Duration::from_millis(800));

        Ok(())
    }
}
