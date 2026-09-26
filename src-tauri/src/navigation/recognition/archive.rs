//! 档案库具体 UI 状态的分层判定。
//!
//! 档案库标题、关闭按钮和分类水印先缩小候选集合，侧边栏颜色再区分见闻辑录和中枢
//! 档案的具体子界面。状态组之间的静态引用表达当前采用的固定执行计划。

use anyhow::Result;
use image::imageops;

use crate::utils::region::{Region2D, ltwh};

use super::{RecognitionContext, Refinement, UiStateGroup};
use crate::navigation::state::{ArchiveState, ArchiveSubscene, CentralPage, RecordsPage, UiState};
use crate::navigation::targets::{
    ARCHIVE_CENTRAL_ENTRY, ARCHIVE_CENTRAL_WATERMARK, ARCHIVE_DETAIL_CLOSE,
    ARCHIVE_DETAIL_DECORATION, ARCHIVE_MAIN_CLOSE, ARCHIVE_MEDIA_ENTRY, ARCHIVE_MEDIA_WATERMARK,
    ARCHIVE_RECORDS_ENTRY, ARCHIVE_RECORDS_WATERMARK, ARCHIVE_SUBSCENE_CLOSE, ARCHIVE_TITLE,
};

/// 档案库导航页状态组，由档案库主界面和全部六个档案库子界面构成。
///
/// 一次细化可以产出具体状态“档案库主界面”，或继续保留状态组
/// “档案库子界面”。关闭按钮和主界面入口均不满足判定条件时产出未识别结果。
pub(super) static ARCHIVE_PAGES: ArchivePages = ArchivePages;
/// 档案库子界面状态组，由音像存档、见闻辑录和中枢档案的六个具体子界面构成。
///
/// 一次细化可以产出具体状态“音像存档 - 多媒体”，或继续保留状态组
/// “见闻辑录子界面”、“中枢档案子界面”。三种分类水印均不匹配时产出未识别结果。
static ARCHIVE_SUBSCENES: ArchiveSubscenes = ArchiveSubscenes;
/// 见闻辑录子界面状态组，由纸质记录、电子档案和藏品构成。
///
/// 一次细化会产出具体状态“见闻辑录 - 纸质记录”、“见闻辑录 - 电子档案”
/// 或“见闻辑录 - 藏品”，不会继续保留状态组。侧边栏均不呈深色时回退到
/// “见闻辑录 - 纸质记录”。
static RECORDS_SUBSCENES: RecordsSubscenes = RecordsSubscenes;
/// 中枢档案子界面状态组，由中枢档案和调查报告构成。
///
/// 一次细化会产出具体状态“中枢档案 - 中枢档案”或“中枢档案 - 调查报告”，
/// 不会继续保留状态组。侧边栏均不呈深色时回退到“中枢档案 - 中枢档案”。
static CENTRAL_SUBSCENES: CentralSubscenes = CentralSubscenes;

/// 子界面侧边栏前三个页签的灰度取样区域，按从上到下的顺序排列。
const TAB_ROIS: [Region2D<u32>; 3] = [
    ltwh!(180, 120, 60, 36),
    ltwh!(180, 184, 60, 36),
    ltwh!(180, 248, 60, 36),
];
/// 判定侧边栏页签呈深色的平均灰度上限，取值范围为 `0..=255`。
const DARK_THRESHOLD: u8 = 128;

pub(super) fn recognizes_archive_title(cx: &mut RecognitionContext<'_>) -> Result<bool> {
    cx.matches(&ARCHIVE_TITLE)
}

pub(super) fn recognize_archive_detail(cx: &mut RecognitionContext<'_>) -> Result<bool> {
    if !cx.matches(&ARCHIVE_DETAIL_DECORATION)? {
        return Ok(false);
    }
    cx.matches(&ARCHIVE_DETAIL_CLOSE)
}

/// 档案库导航页状态组，由档案库主界面和全部六个档案库子界面构成。
///
/// 一次细化可以产出具体状态“档案库主界面”，或继续保留状态组
/// “档案库子界面”。关闭按钮和主界面入口均不满足判定条件时产出未识别结果。
pub(super) struct ArchivePages;

impl UiStateGroup for ArchivePages {
    fn name(&self) -> &'static str {
        "档案库导航页"
    }

    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement> {
        if cx.matches(&ARCHIVE_SUBSCENE_CLOSE)? {
            return Ok(Refinement::StillVague(&ARCHIVE_SUBSCENES));
        }

        if !cx.matches(&ARCHIVE_MAIN_CLOSE)? {
            return Ok(Refinement::Unrecognized);
        }

        let has_entry = cx.matches(&ARCHIVE_MEDIA_ENTRY)?
            || cx.matches(&ARCHIVE_RECORDS_ENTRY)?
            || cx.matches(&ARCHIVE_CENTRAL_ENTRY)?;

        Ok(if has_entry {
            Refinement::Determined(UiState::Archive(ArchiveState::Main))
        } else {
            Refinement::Unrecognized
        })
    }
}

/// 档案库子界面状态组，由音像存档、见闻辑录和中枢档案的六个具体子界面构成。
///
/// 一次细化可以产出具体状态“音像存档 - 多媒体”，或继续保留状态组
/// “见闻辑录子界面”、“中枢档案子界面”。三种分类水印均不匹配时产出未识别结果。
struct ArchiveSubscenes;

impl UiStateGroup for ArchiveSubscenes {
    fn name(&self) -> &'static str {
        "档案库子界面"
    }

    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement> {
        if cx.matches(&ARCHIVE_MEDIA_WATERMARK)? {
            return Ok(Refinement::Determined(UiState::archive_subscene(
                ArchiveSubscene::Media,
            )));
        }
        if cx.matches(&ARCHIVE_RECORDS_WATERMARK)? {
            return Ok(Refinement::StillVague(&RECORDS_SUBSCENES));
        }
        if cx.matches(&ARCHIVE_CENTRAL_WATERMARK)? {
            return Ok(Refinement::StillVague(&CENTRAL_SUBSCENES));
        }
        Ok(Refinement::Unrecognized)
    }
}

/// 见闻辑录子界面状态组，由纸质记录、电子档案和藏品构成。
///
/// 一次细化会产出具体状态“见闻辑录 - 纸质记录”、“见闻辑录 - 电子档案”
/// 或“见闻辑录 - 藏品”，不会继续保留状态组。侧边栏均不呈深色时回退到
/// “见闻辑录 - 纸质记录”。
struct RecordsSubscenes;

impl UiStateGroup for RecordsSubscenes {
    fn name(&self) -> &'static str {
        "见闻辑录子界面"
    }

    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement> {
        let dark = tab_darkness(cx, 3);
        let page = if dark[0] {
            RecordsPage::Paper
        } else if dark[1] {
            RecordsPage::Digital
        } else if dark[2] {
            RecordsPage::Collection
        } else {
            RecordsPage::Paper
        };
        Ok(Refinement::Determined(UiState::archive_subscene(
            ArchiveSubscene::Records(page),
        )))
    }
}

/// 中枢档案子界面状态组，由中枢档案和调查报告构成。
///
/// 一次细化会产出具体状态“中枢档案 - 中枢档案”或“中枢档案 - 调查报告”，
/// 不会继续保留状态组。侧边栏均不呈深色时回退到“中枢档案 - 中枢档案”。
struct CentralSubscenes;

impl UiStateGroup for CentralSubscenes {
    fn name(&self) -> &'static str {
        "中枢档案子界面"
    }

    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement> {
        let dark = tab_darkness(cx, 2);
        let page = if dark[0] {
            CentralPage::Archive
        } else if dark[1] {
            CentralPage::Report
        } else {
            CentralPage::Archive
        };
        Ok(Refinement::Determined(UiState::archive_subscene(
            ArchiveSubscene::Central(page),
        )))
    }
}

fn tab_darkness(cx: &RecognitionContext<'_>, count: usize) -> [bool; 3] {
    let mut result = [false; 3];
    for (index, roi) in TAB_ROIS.iter().take(count).enumerate() {
        let cropped =
            imageops::crop_imm(cx.screenshot, roi.x0(), roi.y0(), roi.width(), roi.height());
        let gray = imageops::grayscale(&cropped.to_image());
        let pixel_count = u64::from(roi.width()) * u64::from(roi.height());
        let total: u64 = gray.pixels().map(|pixel| u64::from(pixel.0[0])).sum();
        result[index] = pixel_count != 0 && total / pixel_count < u64::from(DARK_THRESHOLD);
    }
    result
}
