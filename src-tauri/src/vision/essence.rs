//! 从一张 1280×720 基准截图识别当前详情面板中的基质。
//!
//! 场景判定属于 navigation。这里仅负责详情面板内的属性、等级和状态识别，
//! 并通过 [`TemplateMatching`] 使用会话提供的懒加载模板缓存。

use anyhow::Result;
use image::{RgbaImage, imageops};

use crate::{
    automation::{TemplateMatch, TemplateMatching, TemplateTarget},
    essence::{Essence, Rarity},
    utils::{point::Point2D, region::Region2D},
    vision::region::mean_luma,
};

use super::essence_templates::STAT_TEMPLATES;

const STATS: [Region2D<u32>; 3] = [
    Region2D::from_ltrb(1002, 235, 1138, 263),
    Region2D::from_ltrb(1002, 274, 1138, 301),
    Region2D::from_ltrb(1002, 309, 1138, 336),
];
const LOCK: Region2D<u32> = Region2D::from_ltrb(1214, 177, 1242, 205);
const ABANDON: Region2D<u32> = Region2D::from_ltrb(1190, 177, 1218, 205);
const LEVEL_X: [u32; 6] = [1002, 1013, 1025, 1036, 1047, 1059];
const LEVEL_Y: [u32; 3] = [263, 301, 338];
const TEMPLATE_THRESHOLD: f32 = -1.0;
const SCORE_THRESHOLD: f32 = 0.7;
const AMBIGUITY_MARGIN: f32 = 0.035;

/// 从详情截图识别基质内容。调用方应先用 navigation 确认当前处于基质详情面板。
pub(crate) fn recognize<C: TemplateMatching>(io: &mut C, frame: &RgbaImage) -> Result<Essence> {
    let mut stats = [None, None, None];
    for (index, roi) in STATS.into_iter().enumerate() {
        let mut scores = Vec::with_capacity(STAT_TEMPLATES.len());
        for &(stat_id, path) in STAT_TEMPLATES {
            let target = TemplateTarget {
                template_name: path,
                roi,
                threshold: TEMPLATE_THRESHOLD,
            };
            if let Some(TemplateMatch { score, .. }) = io.find_template(frame, &target)? {
                scores.push((stat_id, score));
            }
        }
        scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        if let Some(&(name, score)) = scores.first()
            && score >= SCORE_THRESHOLD
            && scores
                .get(1)
                .is_none_or(|runner_up| score - runner_up.1 >= AMBIGUITY_MARGIN)
        {
            stats[index] = Some(name.to_owned());
        }
    }
    Ok(Essence {
        stats,
        levels: LEVEL_Y.map(|y| recognize_level(frame, y)),
        rarity: recognize_rarity(frame),
        locked: recognize_marker(io, frame, LOCK, "基质/locked.png", "基质/unlocked.png")?,
        abandoned: recognize_marker(
            io,
            frame,
            ABANDON,
            "基质/abandoned.png",
            "基质/unabandoned.png",
        )?,
    })
}

fn recognize_marker<C: TemplateMatching>(
    io: &mut C,
    frame: &RgbaImage,
    roi: Region2D<u32>,
    yes: &'static str,
    no: &'static str,
) -> Result<Option<bool>> {
    let positive = score(io, frame, roi, yes)?;
    let negative = score(io, frame, roi, no)?;
    Ok(
        (positive.max(negative) >= SCORE_THRESHOLD && (positive - negative).abs() >= 0.04)
            .then_some(positive > negative),
    )
}

fn score<C: TemplateMatching>(
    io: &mut C,
    frame: &RgbaImage,
    roi: Region2D<u32>,
    name: &'static str,
) -> Result<f32> {
    Ok(io
        .find_template(
            frame,
            &TemplateTarget {
                template_name: name,
                roi,
                threshold: TEMPLATE_THRESHOLD,
            },
        )?
        .map_or(-1.0, |m| m.score))
}

fn recognize_level(frame: &RgbaImage, y: u32) -> Option<u8> {
    let count = LEVEL_X
        .into_iter()
        .take_while(|&x| {
            mean_luma(frame, Region2D::from_ltwh(x - 1, y - 1, 3, 3)).unwrap_or(0.0) > 200.0
        })
        .count() as u8;
    (count > 0).then_some(count)
}

fn recognize_rarity(frame: &RgbaImage) -> Rarity {
    let mut rgb = [0.0; 3];
    for y in 52..55 {
        for x in 979..982 {
            for (value, channel) in rgb.iter_mut().zip(frame.get_pixel(x, y).0) {
                *value += f64::from(channel) / 9.0;
            }
        }
    }
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if max < 50.0 {
        return Rarity::Unknown;
    }
    if delta < 40.0 {
        return Rarity::Other;
    }
    let hue = if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    if (hue - 44.0).abs() <= 15.0 {
        Rarity::Five
    } else if (hue - 264.0).abs() <= 15.0 {
        Rarity::Four
    } else {
        Rarity::Other
    }
}

/// 判断背包网格中的一个槽位是否有基质。
pub(crate) fn occupied(frame: &RgbaImage, x: u32, y: u32) -> bool {
    mean_luma(frame, Region2D::from_ltwh(x - 22, y - 25, 44, 44)).is_some_and(|mean| mean >= 38.0)
}

/// 判断滚动条指定位置是否显示亮色滑块。
pub(crate) fn scrollbar_at(frame: &RgbaImage, point: Point2D<u32>) -> bool {
    (point.y.saturating_sub(1)..=point.y + 1).any(|y| {
        (point.x - 3..=point.x + 3).any(|x| frame.get_pixel(x, y).0[..3].iter().all(|&v| v > 100))
    })
}

/// 判断基质列表滚动条是否可见。
pub(crate) fn scrollbar_visible(frame: &RgbaImage) -> bool {
    (87..=633).any(|y| (966..=972).any(|x| frame.get_pixel(x, y).0[..3].iter().all(|&v| v > 100)))
}

/// 判断滚动条轨道在两帧之间是否保持稳定。
pub(crate) fn scrollbar_unchanged(before: &RgbaImage, after: &RgbaImage) -> bool {
    (85..635).all(|y| {
        (966..=972).all(|x| {
            before.get_pixel(x, y).0[..3]
                .iter()
                .zip(&after.get_pixel(x, y).0[..3])
                .all(|(a, b)| a.abs_diff(*b) < 15)
        })
    })
}

/// 截取详情面板区域，供扫描结果保存为预览图。
pub(crate) fn detail_image(frame: &RgbaImage) -> RgbaImage {
    imageops::crop_imm(frame, 974, 45, 286, 315).to_image()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::vision::template_matching::{self, LazyTemplateLoader};

    struct ResourceMatcher {
        loader: LazyTemplateLoader,
    }

    impl ResourceMatcher {
        fn new() -> Self {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/templates");
            Self {
                loader: LazyTemplateLoader::new(root),
            }
        }
    }

    impl TemplateMatching for ResourceMatcher {
        fn find_template(
            &mut self,
            screenshot: &RgbaImage,
            target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>> {
            let matched = template_matching::find(
                screenshot,
                target.template_name,
                target.roi,
                &mut self.loader,
            )?;
            Ok(
                (matched.score >= target.threshold).then_some(TemplateMatch {
                    region: matched.region,
                    score: matched.score,
                }),
            )
        }
    }

    #[test]
    fn recognizes_stats_markers_and_levels_from_real_templates() {
        let mut matcher = ResourceMatcher::new();
        let mut frame = RgbaImage::from_pixel(1280, 720, image::Rgba([0, 0, 0, 255]));
        for (id, roi) in [
            ("gat_passive_attr_str", STATS[0]),
            ("gat_passive_attr_atk", STATS[1]),
            ("gst_passive_tactic", STATS[2]),
            ("unlocked", LOCK),
            ("unabandoned", ABANDON),
        ] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources/templates/基质")
                .join(format!("{id}.png"));
            let template = image::open(path).unwrap().to_rgba8();
            image::imageops::replace(
                &mut frame,
                &template,
                i64::from(roi.x0()),
                i64::from(roi.y0()),
            );
        }
        for &y in &LEVEL_Y {
            for &x in &LEVEL_X[..3] {
                for py in y - 1..=y + 1 {
                    for px in x - 1..=x + 1 {
                        frame.put_pixel(px, py, image::Rgba([230, 230, 230, 255]));
                    }
                }
            }
        }
        for y in 52..55 {
            for x in 979..982 {
                frame.put_pixel(x, y, image::Rgba([230, 170, 40, 255]));
            }
        }
        let essence = recognize(&mut matcher, &frame).unwrap();
        assert_eq!(essence.stats[0].as_deref(), Some("gat_passive_attr_str"));
        assert_eq!(essence.stats[1].as_deref(), Some("gat_passive_attr_atk"));
        assert_eq!(essence.stats[2].as_deref(), Some("gst_passive_tactic"));
        assert_eq!(essence.levels, [Some(3), Some(3), Some(3)]);
        assert_eq!(essence.rarity, Rarity::Five);
        assert_eq!(essence.locked, Some(false));
        assert_eq!(essence.abandoned, Some(false));
    }

    #[test]
    fn leaves_unknown_stats_when_no_template_matches() {
        let mut matcher = ResourceMatcher::new();
        let frame = RgbaImage::from_pixel(1280, 720, image::Rgba([0, 0, 0, 255]));
        let essence = recognize(&mut matcher, &frame).unwrap();
        assert_eq!(essence.stats, [None, None, None]);
        assert_eq!(essence.rarity, Rarity::Unknown);
        assert_eq!(essence.locked, None);
        assert_eq!(essence.abandoned, None);
    }
}
