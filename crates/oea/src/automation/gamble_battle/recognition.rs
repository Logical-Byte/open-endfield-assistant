//! 百趣演武铭牌识别。所有坐标都基于会话提供的 `1280×720` 截图。

use std::path::Path;

use anyhow::{Context, ensure};
use image::{RgbImage, RgbaImage};

use super::model::{Card, Suit};
use crate::{
    utils::region::Region2D,
    vision::{ImageRegion, statistics, template_matching::pure},
};

/// 只统计牌头空白区域的亮度，不加载或匹配数字、花色模板。
pub(super) fn count_cards(screenshot: &RgbaImage) -> anyhow::Result<usize> {
    ensure!(
        screenshot.dimensions() == (1280, 720),
        "百趣演武识别需要 1280×720 截图"
    );
    let mut count = 0;
    let mut empty_seen = false;
    for slot in 0..5 {
        let x = 170 + slot * 184;
        let region = ImageRegion::new(screenshot, Region2D::from_ltrb(x, 165, x + 45, 180))?;
        let luma = statistics::mean_luma(&region);
        // 图包中牌面约 239、牌背约 140，独立视频的变暗牌面最低约 183。
        // 阈值间留出不确定区间，弹窗遮罩或翻牌过渡交给工作流重试。
        if luma >= 175.0 {
            ensure!(!empty_seen, "百趣演武铭牌未按顺序翻开");
            count += 1;
        } else {
            ensure!(
                (120.0..=155.0).contains(&luma),
                "第 {} 个牌槽亮度不明确：{luma}",
                slot + 1
            );
            empty_seen = true;
        }
    }
    Ok(count)
}

pub(super) struct Recognizer {
    ranks: [RgbImage; 5],
    suits: [(Suit, RgbImage); 3],
    empty: RgbImage,
}

impl Recognizer {
    pub(super) fn new(templates_dir: &Path) -> anyhow::Result<Self> {
        let load = |name: &str| -> anyhow::Result<RgbImage> {
            let path = templates_dir.join("百趣演武").join(name);
            Ok(image::open(&path)
                .with_context(|| format!("加载百趣演武模板失败：{}", path.display()))?
                .to_rgb8())
        };
        Ok(Self {
            ranks: [
                load("rank-1.png")?,
                load("rank-2.png")?,
                load("rank-3.png")?,
                load("rank-4.png")?,
                load("rank-5.png")?,
            ],
            suits: [
                (Suit::Bing, load("bing.png")?),
                (Suit::Tiao, load("tiao.png")?),
                (Suit::Wan, load("wan.png")?),
            ],
            empty: load("empty.png")?,
        })
    }

    pub(super) fn read_cards(&self, screenshot: &RgbaImage) -> anyhow::Result<Vec<Card>> {
        ensure!(
            screenshot.dimensions() == (1280, 720),
            "百趣演武识别需要 1280×720 截图"
        );
        let mut cards = Vec::with_capacity(5);
        let mut empty_seen = false;
        for slot in 0..5 {
            let x = 105 + slot * 184;
            let empty_region =
                ImageRegion::new(screenshot, Region2D::from_ltrb(x + 10, 269, x + 130, 305))?;
            if pure::match_template(&empty_region, &self.empty)?.score >= 0.85 {
                empty_seen = true;
                continue;
            }
            // 未识别的牌头不是空槽，动画或未知画面交由工作流重试。
            ensure!(!empty_seen, "百趣演武铭牌未按顺序翻开");
            let rank_region =
                ImageRegion::new(screenshot, Region2D::from_ltrb(x + 6, 165, x + 39, 205))?;
            let rank_scores = self
                .ranks
                .iter()
                .map(|template| {
                    pure::match_template(&rank_region, template).map(|matched| matched.score)
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let rank = best_candidate(&rank_scores)
                .with_context(|| format!("第 {} 张铭牌战力点不明确", slot + 1))?
                as u8
                + 1;
            let suit_region =
                ImageRegion::new(screenshot, Region2D::from_ltrb(x + 38, 174, x + 69, 205))?;
            let suit_scores = self
                .suits
                .iter()
                .map(|(_, template)| {
                    pure::match_template(&suit_region, template).map(|matched| matched.score)
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let suit = self.suits[best_candidate(&suit_scores)
                .with_context(|| format!("第 {} 张铭牌花色不明确", slot + 1))?]
            .0;
            cards.push(Card { suit, rank });
        }
        Ok(cards)
    }
}

/// 同时要求绝对置信度和候选间隔，避免把未翻开或动画中的牌猜成有效结果。
fn best_candidate(scores: &[f32]) -> anyhow::Result<usize> {
    let mut order: Vec<_> = scores.iter().copied().enumerate().collect();
    order.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
    ensure!(
        order[0].1 >= 0.8 && order[0].1 - order[1].1 >= 0.08,
        "模板匹配分数不足或候选相近：{scores:?}"
    );
    Ok(order[0].0)
}
