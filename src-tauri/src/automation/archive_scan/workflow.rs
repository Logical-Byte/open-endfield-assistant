//! 档案库扫描工作流。

use anyhow::Result;
use tracing::info;

use crate::{
    automation::{Clock, Input, Ocr, ScreenCapture, TemplateMatching},
    data::ArchiveTitleIndex,
    navigation::{ArchiveState, Navigator, UiState},
};

use super::{
    correction::{CorrectionOverride, DEFAULT_CORRECTION_OVERRIDES},
    plan::SCAN_PLAN,
    reporting::ScanReporter,
    scan_loop::scan_current_sub_scene,
};

/// 档案库扫描工作流：扫描全部 6 个子分类的档案。
///
/// 扫描结果上报器与档案标题索引由工作者注入。
pub(super) struct ArchiveScanner<'a> {
    reporter: ScanReporter,
    archive_titles: &'a ArchiveTitleIndex,
    correction_overrides: Option<&'a [CorrectionOverride<'a>]>,
}

impl<'a> ArchiveScanner<'a> {
    /// 创建扫描工作流。
    pub(super) fn new(reporter: ScanReporter, archive_titles: &'a ArchiveTitleIndex) -> Self {
        Self {
            reporter,
            archive_titles,
            correction_overrides: Some(DEFAULT_CORRECTION_OVERRIDES),
        }
    }

    /// 移动鼠标、进入档案库主界面，然后扫描全部子分类。
    pub(super) fn run<C>(&self, cx: &mut C, navigator: &Navigator) -> Result<()>
    where
        C: ScreenCapture + Input + TemplateMatching + Ocr + Clock,
    {
        info!("========== 开始执行任务: 扫描档案库 ==========");

        // 避免鼠标 hover 样式变化干扰首次 UI 状态识别和导航。
        cx.move_mouse_to_safe_position()?;
        navigator.navigate_to(UiState::Archive(ArchiveState::Main), cx)?;

        for (index, &subscene) in SCAN_PLAN.iter().enumerate() {
            info!(
                "===== 扫描子分类 {}/{}: {} =====",
                index + 1,
                SCAN_PLAN.len(),
                subscene
            );
            navigator.navigate_to(UiState::archive_subscene(subscene), cx)?;
            scan_current_sub_scene(
                cx,
                navigator,
                subscene,
                self.archive_titles,
                self.correction_overrides,
                &self.reporter,
            )?;
            info!("完成扫描 {subscene}");
        }

        info!("全部 6 个子分类扫描完毕！");
        info!("========== 任务 扫描档案库 执行完毕 ==========");
        Ok(())
    }
}
