use std::{sync::Arc, time::Duration};

use image::{Rgba, RgbaImage};

use crate::{
    app_paths::AppPaths,
    automation::{
        self, Clock, Drag, Input, Key, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget, capabilities, events::testing::RecordingEventSink,
    },
    data::essence,
};

use super::{ReportStore, layout, reporting::Reporter, workflow};
use crate::vision::{ImageRegion, template_matching};

/// 游戏能力的脚本 adapter。第一行仅有两枚内容相同的基质，其他位置为空。
struct Inventory {
    frame: RgbaImage,
    clicks: Vec<Point720p>,
    count: usize,
    offset: u32,
    stop_after: Option<usize>,
    matcher: Option<template_matching::LazyTemplateLoader>,
}

impl Inventory {
    fn two_identical_items() -> Self {
        let mut frame = RgbaImage::from_pixel(1280, 720, Rgba([10, 10, 10, 255]));
        for x in &layout::COLUMNS[..2] {
            for px in x - 22..x + 22 {
                for py in 106..150 {
                    frame.put_pixel(px, py, Rgba([90, 90, 90, 255]));
                }
            }
        }
        // EER 顶部/底部滚动条亮点，表示一屏即可遍历完整。
        for y in [layout::SCROLL_TOP, layout::SCROLL_BOTTOM] {
            frame.put_pixel(layout::SCROLL_X, y, Rgba([220, 220, 220, 255]));
        }
        Self {
            frame,
            clicks: Vec::new(),
            count: 2,
            offset: 0,
            stop_after: None,
            matcher: None,
        }
    }

    fn scrolling(count: usize) -> Self {
        let mut io = Self::two_identical_items();
        io.count = count;
        io.frame = io.scroll_frame();
        io
    }

    fn scroll_frame(&self) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(1280, 720, Rgba([10, 10, 10, 255]));
        for index in 0..self.count {
            let x = layout::COLUMNS[index % 9];
            let y = layout::row_y((index / 9) as u32, self.offset);
            for py in (y - 40).max(97)..(y + 40).min(620) {
                for px in x - 40..x + 40 {
                    // 随内容滚动的图案，允许实际位移测量。基质详情仍然相同。
                    let seed = px.wrapping_mul(1664525)
                        ^ (py as u32 + self.offset).wrapping_mul(1013904223);
                    let gray = 60 + ((seed >> 16) % 150) as u8;
                    image.put_pixel(px, py as u32, Rgba([gray, gray, gray, 255]));
                }
            }
        }
        let bottom = self.max_offset();
        let y = if self.offset == 0 {
            layout::SCROLL_TOP
        } else if self.offset == bottom {
            layout::SCROLL_BOTTOM
        } else {
            360
        };
        image.put_pixel(layout::SCROLL_X, y, Rgba([220, 220, 220, 255]));
        image
    }

    fn max_offset(&self) -> u32 {
        (layout::row_y(((self.count - 1) / 9) as u32, 0) - 547).max(0) as u32
    }
}

impl ScreenCapture for Inventory {
    fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
        Ok(self.frame.clone())
    }
}

impl Input for Inventory {
    fn click(&mut self, point: Point720p) -> Result<(), capabilities::Error> {
        if self.stop_after == Some(self.clicks.len()) {
            return Err(capabilities::Error::StoppedByUser);
        }
        self.clicks.push(point);
        Ok(())
    }
    fn press_key(&mut self, _: Key) -> Result<(), capabilities::Error> {
        unreachable!()
    }
    fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
        Ok(())
    }
}

impl Clock for Inventory {
    fn sleep(&mut self, _: Duration) {}
}

impl Drag for Inventory {
    fn drag(&mut self, from: Point720p, to: Point720p) -> Result<(), capabilities::Error> {
        self.offset = if from.y < to.y {
            0
        } else {
            (self.offset + from.y - to.y).min(self.max_offset())
        };
        self.frame = self.scroll_frame();
        Ok(())
    }
}

impl TemplateMatching for Inventory {
    fn find_template(
        &mut self,
        _: &RgbaImage,
        target: &TemplateTarget<'_>,
    ) -> Result<Option<TemplateMatch>, capabilities::Error> {
        if let Some(loader) = &mut self.matcher {
            let matched = template_matching::find(
                &ImageRegion::new(&self.frame, target.roi).unwrap(),
                target.template_name,
                loader,
            )
            .unwrap();
            return Ok(
                (matched.score >= target.threshold).then_some(TemplateMatch {
                    region: matched.region,
                    score: matched.score,
                }),
            );
        }
        // 页面标题已确认，详情故意无法识别，验证部分结果不丢失。
        Ok(
            (target.template_name == "基质/武器基质.png").then_some(TemplateMatch {
                region: target.roi,
                score: 1.0,
            }),
        )
    }
}

#[test]
fn stopping_keeps_the_records_observed_before_the_stop() {
    let data = essence::Database::load(&AppPaths::new().unwrap()).unwrap();
    let store = Arc::new(ReportStore::default());
    let reporter = Reporter::begin(
        Arc::clone(&store),
        Arc::new(RecordingEventSink::default()),
        automation::WorkerType::Simulation,
    );
    let mut io = Inventory::two_identical_items();
    io.stop_after = Some(1);
    let result = workflow::scan(&mut io, data.catalog(), &reporter);
    reporter.finish(&result);
    let report = store.snapshot().unwrap();
    assert_eq!(report.records.len(), 1);
    assert!(matches!(
        report.progress.outcome,
        Some(super::model::Outcome::Stopped)
    ));
}

#[test]
fn reads_real_eer_templates_and_preserves_the_semantic_types_in_screen_order() {
    let paths = AppPaths::new().unwrap();
    let data = essence::Database::load(&paths).unwrap();
    let store = Arc::new(ReportStore::default());
    let reporter = Reporter::begin(
        Arc::clone(&store),
        Arc::new(RecordingEventSink::default()),
        automation::WorkerType::Simulation,
    );
    let mut io = Inventory::two_identical_items();
    let template_root = paths.templates_dir();
    for (name, roi) in [
        ("武器基质", layout::TITLE),
        ("gst_passive_tactic", layout::STATS[0]),
        ("gat_passive_attr_atk", layout::STATS[1]),
        ("gat_passive_attr_str", layout::STATS[2]),
        ("未锁定", layout::LOCK),
        ("已弃用", layout::ABANDON),
    ] {
        let template = image::open(template_root.join("基质").join(format!("{name}.png")))
            .unwrap()
            .to_rgba8();
        image::imageops::replace(
            &mut io.frame,
            &template,
            i64::from(roi.x0()),
            i64::from(roi.y0()),
        );
    }
    for (index, y) in layout::LEVEL_Y.into_iter().enumerate() {
        for x in &layout::LEVEL_X[..[2, 4, 6][index]] {
            for px in x - 1..=x + 1 {
                for py in y - 1..=y + 1 {
                    io.frame.put_pixel(px, py, Rgba([230, 230, 230, 255]));
                }
            }
        }
    }
    for y in layout::RARITY.y0()..layout::RARITY.y1() {
        for x in layout::RARITY.x0()..layout::RARITY.x1() {
            io.frame.put_pixel(x, y, Rgba([255, 186, 3, 255]));
        }
    }
    io.matcher = Some(template_matching::LazyTemplateLoader::new(template_root));
    let result = workflow::scan(&mut io, data.catalog(), &reporter);
    reporter.finish(&result);
    assert!(result.is_ok(), "{result:?}");
    let report = store.snapshot().unwrap();
    let record = &report.records[0];
    assert_eq!(
        record
            .terms
            .iter()
            .map(|term| (term.term_id.as_ref().map(|id| id.as_str()), term.level))
            .collect::<Vec<_>>(),
        vec![
            (Some("gst_passive_tactic"), Some(2)),
            (Some("gat_passive_attr_atk"), Some(4)),
            (Some("gat_passive_attr_str"), Some(6))
        ]
    );
    assert_eq!(record.terms[0].kind, Some(essence::TermKind::Skill));
    assert_eq!(record.rarity, super::model::Rarity::Five);
    assert_eq!(record.locked, Some(false));
    assert_eq!(record.abandoned, Some(true));
    assert!(record.issues.is_empty(), "{:?}", record.issues);
}

#[test]
fn reports_identical_physical_items_separately_and_keeps_unknown_fields() {
    let data = essence::Database::load(&AppPaths::new().unwrap()).unwrap();
    let store = Arc::new(ReportStore::default());
    let reporter = Reporter::begin(
        Arc::clone(&store),
        Arc::new(RecordingEventSink::default()),
        automation::WorkerType::Simulation,
    );
    let mut io = Inventory::two_identical_items();
    let result = workflow::scan(&mut io, data.catalog(), &reporter);
    reporter.finish(&result);
    assert!(result.is_ok());
    let report = store.snapshot().unwrap();
    assert_eq!(report.records.len(), 2);
    assert_eq!(report.progress.incomplete_count, 2);
    assert_eq!(
        report
            .records
            .iter()
            .map(|r| (r.record_id, r.row, r.column))
            .collect::<Vec<_>>(),
        vec![(1, 1, 1), (2, 1, 2)]
    );
    assert!(
        report
            .records
            .iter()
            .all(|r| r.locked.is_none() && r.terms[0].level.is_none())
    );
    assert_eq!(io.clicks.len(), 2);
}

#[test]
fn follows_observed_scroll_positions_through_a_partial_last_row() {
    let data = essence::Database::load(&AppPaths::new().unwrap()).unwrap();
    let store = Arc::new(ReportStore::default());
    let reporter = Reporter::begin(
        Arc::clone(&store),
        Arc::new(RecordingEventSink::default()),
        automation::WorkerType::Simulation,
    );
    let mut io = Inventory::scrolling(65);
    let result = workflow::scan(&mut io, data.catalog(), &reporter);
    reporter.finish(&result);
    assert!(result.is_ok(), "{result:?}");
    let report = store.snapshot().unwrap();
    assert_eq!(report.records.len(), 65);
    assert_eq!(
        report
            .records
            .iter()
            .map(|r| (r.row, r.column))
            .collect::<Vec<_>>(),
        (0..65).map(|i| (i / 9 + 1, i % 9 + 1)).collect::<Vec<_>>()
    );
    assert!(matches!(
        report.progress.outcome,
        Some(super::model::Outcome::Completed)
    ));
}
