//! 从单张 720p 截图识别基质。模板随程序编译，使用 OEA 的相关系数匹配实现。

use anyhow::{Result, ensure};
use image::{RgbImage, RgbaImage};

use crate::{
    essence::{Essence, Rarity},
    utils::region::Region2D,
    vision::{region::mean_luma, template_matching::pure::match_template_in_region},
};

use super::{layout, templates::TEMPLATES};

pub(super) struct Recognizer {
    templates: Vec<(&'static str, RgbImage)>,
}

impl Recognizer {
    pub(super) fn new() -> Result<Self> {
        let templates = TEMPLATES
            .iter()
            .map(|(name, bytes)| Ok((*name, image::load_from_memory(bytes)?.to_rgb8())))
            .collect::<Result<_>>()?;
        Ok(Self { templates })
    }

    pub(super) fn check_scene(&self, image: &RgbaImage) -> Result<()> {
        ensure!(
            image.dimensions() == (1280, 720),
            "基质扫描只支持 1280×720 客户区"
        );
        ensure!(
            self.score(image, "scene", layout::SCENE)? >= 0.7,
            "未识别到武器基质页面，请按 N 打开贵重品库并选择武器基质"
        );
        Ok(())
    }

    pub(super) fn recognize(&self, image: &RgbaImage) -> Result<Essence> {
        self.check_scene(image)?;
        let mut stats = [None, None, None];
        for (index, roi) in layout::STATS.into_iter().enumerate() {
            let mut scores = self
                .templates
                .iter()
                .filter(|(id, _)| id.starts_with("gat_") || id.starts_with("gst_"))
                .map(|(id, template)| {
                    Ok((
                        *id,
                        match_template_in_region(image, template, Some(roi))?.score,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            // 相似属性名之间的分数接近时保留未知，交给结果页展示待确认。
            if let Some(&(id, score)) = scores.first()
                && score >= 0.7
                && scores
                    .get(1)
                    .is_none_or(|runner_up| score - runner_up.1 >= 0.035)
            {
                stats[index] = Some(id.to_owned());
            }
        }
        Ok(Essence {
            stats,
            levels: layout::LEVEL_Y.map(|y| recognize_level(image, y)),
            rarity: recognize_rarity(image),
            locked: self.marker(image, layout::LOCK, "locked", "unlocked")?,
            abandoned: self.marker(image, layout::ABANDON, "abandoned", "unabandoned")?,
        })
    }

    fn marker(
        &self,
        image: &RgbaImage,
        roi: Region2D<u32>,
        yes: &str,
        no: &str,
    ) -> Result<Option<bool>> {
        let positive = self.score(image, yes, roi)?;
        let negative = self.score(image, no, roi)?;
        Ok(
            (positive.max(negative) >= 0.7 && (positive - negative).abs() >= 0.04)
                .then_some(positive > negative),
        )
    }

    fn score(&self, image: &RgbaImage, name: &str, roi: Region2D<u32>) -> Result<f32> {
        let (_, template) = self
            .templates
            .iter()
            .find(|(id, _)| *id == name)
            .expect("内置模板名称应存在");
        Ok(match_template_in_region(image, template, Some(roi))?.score)
    }
}

fn recognize_level(image: &RgbaImage, y: u32) -> Option<u8> {
    let count = layout::LEVEL_X
        .into_iter()
        .take_while(|&x| {
            mean_luma(image, Region2D::from_ltwh(x - 1, y - 1, 3, 3)).unwrap_or(0.0) > 200.0
        })
        .count() as u8;
    (count > 0).then_some(count)
}

fn recognize_rarity(image: &RgbaImage) -> Rarity {
    let mut rgb = [0.0; 3];
    for y in 52..55 {
        for x in 979..982 {
            for (value, channel) in rgb.iter_mut().zip(image.get_pixel(x, y).0) {
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

/// 空槽只有暗色背景。只检查卡片内部，避开选中边框和锁定角标。
pub(super) fn occupied(image: &RgbaImage, x: u32, y: u32) -> bool {
    let roi = Region2D::from_ltwh(x - 22, y - 25, 44, 44);
    mean_luma(image, roi).is_some_and(|mean| mean >= 38.0)
}

pub(super) fn scrollbar_at(image: &RgbaImage, point: crate::automation::Point720p) -> bool {
    (point.y.saturating_sub(1)..=point.y + 1).any(|y| {
        (point.x - 3..=point.x + 3).any(|x| image.get_pixel(x, y).0[..3].iter().all(|&v| v > 100))
    })
}

pub(super) fn scrollbar_visible(image: &RgbaImage) -> bool {
    (87..=633).any(|y| (966..=972).any(|x| image.get_pixel(x, y).0[..3].iter().all(|&v| v > 100)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn counts_contiguous_level_lights_and_leaves_blank_unknown() {
        let mut image = RgbaImage::new(1280, 720);
        let y = layout::LEVEL_Y[0];
        assert_eq!(recognize_level(&image, y), None);
        for &x in &layout::LEVEL_X[..3] {
            for py in y - 1..=y + 1 {
                for px in x - 1..=x + 1 {
                    image.put_pixel(px, py, Rgba([230, 230, 230, 255]));
                }
            }
        }
        // 后方零散亮点不能越过断点继续计数。
        image.put_pixel(layout::LEVEL_X[5], y, Rgba([255; 4]));
        assert_eq!(recognize_level(&image, y), Some(3));
    }

    #[test]
    fn recognizes_embedded_templates_in_a_synthetic_detail_panel() {
        let recognizer = Recognizer::new().unwrap();
        let mut image = RgbaImage::new(1280, 720);
        let placements = [
            ("scene", layout::SCENE),
            ("unlocked", layout::LOCK),
            ("abandoned", layout::ABANDON),
            ("gat_passive_attr_str", layout::STATS[0]),
            ("gat_passive_attr_atk", layout::STATS[1]),
            ("gst_passive_tactic", layout::STATS[2]),
        ];
        for (name, roi) in placements {
            let (_, template) = recognizer
                .templates
                .iter()
                .find(|(id, _)| *id == name)
                .unwrap();
            image::imageops::replace(
                &mut image,
                &image::DynamicImage::ImageRgb8(template.clone()).to_rgba8(),
                i64::from(roi.x0()),
                i64::from(roi.y0()),
            );
        }
        let essence = recognizer.recognize(&image).unwrap();
        assert_eq!(essence.stats[0].as_deref(), Some("gat_passive_attr_str"));
        assert_eq!(essence.stats[1].as_deref(), Some("gat_passive_attr_atk"));
        assert_eq!(essence.stats[2].as_deref(), Some("gst_passive_tactic"));
        assert_eq!(essence.locked, Some(false));
        assert_eq!(essence.abandoned, Some(true));
        assert_eq!(essence.levels, [None; 3]);
    }
}
