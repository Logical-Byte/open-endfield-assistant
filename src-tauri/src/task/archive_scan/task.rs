//! 档案库扫描任务定义。

use anyhow::Result;
use tracing::info;

use crate::{
    automation::{Clock, Input, Ocr, ScreenCapture, TemplateMatching},
    navigation::{ArchiveState, Navigator, UiState},
    task::Task,
};

use super::correction::{CorrectionOverride, DEFAULT_CORRECTION_OVERRIDES};
use super::plan::SCAN_PLAN;
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

    fn precondition_state(&self) -> UiState {
        UiState::Archive(ArchiveState::Main)
    }

    fn run<C>(&self, cx: &mut C, navigator: &Navigator) -> Result<()>
    where
        C: ScreenCapture + Input + TemplateMatching + Ocr + Clock,
    {
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
        Ok(())
    }
}
