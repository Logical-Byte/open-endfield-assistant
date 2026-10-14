//! EER `core/recognition/tasks/` 的基质识别移植，领域解释留在扫描模块。

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{RgbaImage, imageops};

use crate::{
    automation::{self, TemplateMatching, TemplateTarget},
    data::essence,
    utils::region::Region2D,
    vision::{ImageRegion, color::Hsv, statistics},
};

use super::{
    layout,
    model::{Rarity, RecognitionIssue, ScanRecord, TermObservation},
};

// EER attribute.py、abandon_lock_status.py：0.75 高置信阈值，0.50 低置信阈值。
// 本次普查仅接受高置信匹配，低置信保持未知。额外 0.035 差值为 OEA 保守歧义策略，待实机校准。
const SCORE_THRESHOLD: f32 = 0.75;
const AMBIGUITY_MARGIN: f32 = 0.035;

fn region(frame: &RgbaImage, roi: Region2D<u32>) -> ImageRegion<'_, RgbaImage> {
    ImageRegion::new(frame, roi).expect("布局已限定在 1280×720 截图内")
}

pub(super) fn require_inventory(
    io: &mut impl TemplateMatching,
    frame: &RgbaImage,
) -> Result<(), automation::WorkerError> {
    if frame.dimensions() != (1280, 720) {
        return Err(failure("基质扫描需要 1280×720 基准截图"));
    }
    if io
        .find_template(
            frame,
            &TemplateTarget {
                template_name: "基质/武器基质.png",
                roi: layout::TITLE,
                threshold: SCORE_THRESHOLD,
            },
        )?
        .is_none()
    {
        return Err(failure("请进入简体中文的武器基质背包，清除筛选后重试"));
    }
    Ok(())
}

pub(super) fn recognize(
    io: &mut impl TemplateMatching,
    frame: &RgbaImage,
    catalog: &essence::Catalog,
    row: u32,
    column: u32,
) -> Result<ScanRecord, automation::WorkerError> {
    let mut terms = Vec::new();
    let mut issues = Vec::new();
    for (index, roi) in layout::STATS.into_iter().enumerate() {
        let mut candidates = Vec::new();
        for term in &catalog.terms {
            let name = format!("基质/{}.png", term.id.as_str());
            if let Some(matched) = io.find_template(
                frame,
                &TemplateTarget {
                    template_name: &name,
                    roi,
                    threshold: -1.0,
                },
            )? {
                candidates.push((term, matched.score));
            }
        }
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        let term = candidates
            .first()
            .filter(|(_, score)| {
                *score >= SCORE_THRESHOLD
                    && candidates
                        .get(1)
                        .is_none_or(|(_, second)| *score - *second >= AMBIGUITY_MARGIN)
            })
            .map(|(term, _)| *term);
        let kind = term.map(|term| term.kind);
        let mut level = recognize_level(frame, layout::LEVEL_Y[index]);
        if kind == Some(essence::TermKind::Skill) && level.is_some_and(|level| level > 3) {
            level = None;
        }
        let position = index as u8 + 1;
        if term.is_none() {
            issues.push(RecognitionIssue::Term { position });
        }
        if level.is_none() {
            issues.push(RecognitionIssue::Level { position });
        }
        terms.push(TermObservation {
            term_id: term.map(|term| term.id.clone()),
            kind,
            level,
        });
    }
    for kind in [
        essence::TermKind::Attribute,
        essence::TermKind::Secondary,
        essence::TermKind::Skill,
    ] {
        if terms.iter().filter(|term| term.kind == Some(kind)).count() > 1 {
            issues.push(RecognitionIssue::TermLayout);
            break;
        }
    }
    let rarity = recognize_rarity(frame);
    let locked = marker(
        io,
        frame,
        layout::LOCK,
        "基质/已锁定.png",
        "基质/未锁定.png",
    )?;
    let abandoned = marker(
        io,
        frame,
        layout::ABANDON,
        "基质/已弃用.png",
        "基质/未弃用.png",
    )?;
    if rarity == Rarity::Unknown {
        issues.push(RecognitionIssue::Rarity);
    }
    if locked.is_none() {
        issues.push(RecognitionIssue::Locked);
    }
    if abandoned.is_none() {
        issues.push(RecognitionIssue::Abandoned);
    }
    let roi = layout::DETAIL;
    let detail =
        imageops::crop_imm(frame, roi.x0(), roi.y0(), roi.width(), roi.height()).to_image();
    let mut bytes = Cursor::new(Vec::new());
    detail
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|_| failure("无法编码基质详情截图"))?;
    Ok(ScanRecord {
        run_id: 0,
        record_id: 0,
        row,
        column,
        terms: terms.try_into().expect("三项词条"),
        rarity,
        locked,
        abandoned,
        issues,
        image: format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes.into_inner())
        ),
    })
}

fn marker(
    io: &mut impl TemplateMatching,
    frame: &RgbaImage,
    roi: Region2D<u32>,
    yes: &str,
    no: &str,
) -> Result<Option<bool>, automation::WorkerError> {
    let mut scores = [-1.0; 2];
    for (index, name) in [yes, no].into_iter().enumerate() {
        if let Some(matched) = io.find_template(
            frame,
            &TemplateTarget {
                template_name: name,
                roi,
                threshold: -1.0,
            },
        )? {
            scores[index] = matched.score;
        }
    }
    Ok((scores[0].max(scores[1]) >= SCORE_THRESHOLD
        && (scores[0] - scores[1]).abs() >= AMBIGUITY_MARGIN)
        .then_some(scores[0] > scores[1]))
}

fn recognize_level(frame: &RgbaImage, y: u32) -> Option<u8> {
    // EER attribute_level.py：亮度阈值 200、1080p 半径 2。720p 使用半径 1 的 3×3 采样。
    let level = layout::LEVEL_X
        .into_iter()
        .take_while(|x| {
            statistics::mean_luma(&region(frame, Region2D::from_ltwh(x - 1, y - 1, 3, 3))) > 200.0
        })
        .count() as u8;
    (level > 0).then_some(level)
}

fn recognize_rarity(frame: &RgbaImage) -> Rarity {
    // EER essence_rarity.py：RGB (255,186,3)/(148,82,250)，色相容差 15°、饱和度下限 50/255。
    let color = Hsv::from_rgb(statistics::mean_rgb(&region(frame, layout::RARITY)));
    if color.value < 50.0 / 255.0 {
        return Rarity::Unknown;
    }
    if color.saturation >= 50.0 / 255.0 {
        for (rarity, rgb) in [
            (Rarity::Five, [255.0, 186.0, 3.0]),
            (Rarity::Four, [148.0, 82.0, 250.0]),
        ] {
            if color.hue_distance(Hsv::from_rgb(rgb)) <= 15.0 {
                return rarity;
            }
        }
    }
    Rarity::Other
}

pub(super) fn occupied(frame: &RgbaImage, x: u32, y: u32) -> bool {
    // OEA 旧 essence-scan 原型的占用启发式：中心 44×44 平均亮度 >=38。
    // EER 按整页网格点击，未提供空槽分类。此阈值与取样区域需 Windows 校准，不能视为 EER 实测常量。
    statistics::mean_luma(&region(frame, Region2D::from_ltwh(x - 22, y - 25, 44, 44))) >= 38.0
}

pub(super) fn scrollbar_at(frame: &RgbaImage, y: u32) -> bool {
    // EER engine.py _scrollbar_check_roi：三个 RGB 通道 >100。搜索半宽 3、半高 1 已换算到 720p。
    statistics::count_rgb_above(
        &region(
            frame,
            Region2D::from_ltwh(layout::SCROLL_X - 3, y - 1, 7, 3),
        ),
        [100; 3],
    ) > 0
}

pub(super) fn scrollbar_visible(frame: &RgbaImage) -> bool {
    statistics::count_rgb_above(
        &region(
            frame,
            Region2D::from_ltrb(
                layout::SCROLL_X - 3,
                layout::SCROLL_TOP - 1,
                layout::SCROLL_X + 4,
                layout::SCROLL_BOTTOM + 2,
            ),
        ),
        [100; 3],
    ) > 0
}

pub(super) fn failure(message: &str) -> automation::WorkerError {
    automation::WorkerError::Custom {
        message: message.to_owned(),
    }
}
