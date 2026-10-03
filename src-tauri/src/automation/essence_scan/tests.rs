use std::{sync::Mutex, time::Duration};

use anyhow::Result;
use image::{Rgba, RgbaImage};

use crate::{
    automation::{
        AutomationStopped, Clock, Drag, Event, EventSink, Input, Key, Point720p, ScreenCapture,
        TemplateMatch, TemplateMatching, TemplateTarget,
    },
    essence::{self, Catalog, ScanSettings, Stat, StatKind, Weapon},
};

use super::{ScannedItem, layout, workflow};

#[derive(Default)]
struct Results(Mutex<Vec<ScannedItem>>);

impl EventSink for Results {
    fn publish(&self, event: Event) {
        if let Event::EssenceItemScanned(item) = event {
            self.0.lock().unwrap().push(item);
        }
    }
}

/// 73 枚基质，9 行，尾页仅剩一格。每格的识别结果相同，仍应按位置各扫一次。
struct Inventory {
    offset: u32,
    clicks: Vec<(u32, u32)>,
    stop_after: Option<usize>,
    freeze_scroll: bool,
}

impl ScreenCapture for Inventory {
    fn screenshot(&mut self) -> Result<RgbaImage> {
        let mut frame = RgbaImage::from_pixel(1280, 720, Rgba([10, 10, 10, 255]));
        for y in 85..632 {
            let world_y = y + self.offset;
            let row =
                ((f64::from(world_y) - layout::FIRST_ROW_Y) / layout::ROW_PITCH).round() as i32;
            if !(0..9).contains(&row) {
                continue;
            }
            let center = layout::row_y(row as u32, 0);
            if (world_y as i32 - center).abs() > 44 {
                continue;
            }
            for (column, center_x) in layout::COLUMNS.into_iter().enumerate() {
                if row * 9 + column as i32 >= 73 {
                    continue;
                }
                for x in center_x - 43..=center_x + 43 {
                    let value = ((x * 73 + world_y * 193) ^ (world_y * 977)) as u8;
                    frame.put_pixel(x, y, Rgba([40 + value / 3, 60 + value / 2, 100, 255]));
                }
            }
        }
        for y in [263, 301, 338] {
            for py in y - 1..=y + 1 {
                for x in 1001..=1003 {
                    frame.put_pixel(x, py, Rgba([255; 4]));
                }
            }
        }
        for y in 52..55 {
            for x in 979..982 {
                frame.put_pixel(x, y, Rgba([255, 186, 3, 255]));
            }
        }
        let marker = if self.offset == 0 {
            87
        } else if self.offset == 400 {
            633
        } else {
            300
        };
        frame.put_pixel(969, marker, Rgba([230; 4]));
        Ok(frame)
    }
}

impl TemplateMatching for Inventory {
    fn find_template(
        &mut self,
        _screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        let name = target
            .template_name
            .rsplit('/')
            .next()
            .unwrap()
            .trim_end_matches(".png");
        let matched = matches!(name, "scene" | "unlocked" | "unabandoned")
            || (name == "gat_passive_attr_str" && target.roi.y0() == 235)
            || (name == "gat_passive_attr_atk" && target.roi.y0() == 274)
            || (name == "gst_passive_tactic" && target.roi.y0() == 309);
        Ok(matched.then_some(TemplateMatch {
            region: target.roi,
            score: 1.0,
        }))
    }
}

impl Input for Inventory {
    fn click(&mut self, point: Point720p) -> Result<()> {
        if self.stop_after == Some(self.clicks.len()) {
            return Err(AutomationStopped.into());
        }
        let row = ((f64::from(point.y + self.offset) - layout::FIRST_ROW_Y) / layout::ROW_PITCH)
            .round() as u32;
        let column = layout::COLUMNS.iter().position(|x| *x == point.x).unwrap() as u32;
        self.clicks.push((row, column));
        Ok(())
    }

    fn press_key(&mut self, _key: Key) -> Result<()> {
        Ok(())
    }
    fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Drag for Inventory {
    fn drag(&mut self, from: Point720p, to: Point720p) -> Result<()> {
        if self.freeze_scroll {
            return Ok(());
        }
        self.offset = if to.y > from.y {
            self.offset.saturating_sub(to.y - from.y)
        } else {
            (self.offset + from.y - to.y).min(400)
        };
        Ok(())
    }
}

impl Clock for Inventory {
    fn sleep(&mut self, _duration: Duration) {}
}

fn catalog() -> Catalog {
    let stats = [
        ("gat_passive_attr_str", StatKind::Attribute),
        ("gat_passive_attr_atk", StatKind::Secondary),
        ("gst_passive_tactic", StatKind::Skill),
    ];
    Catalog {
        stats: stats
            .map(|(id, kind)| Stat {
                id: id.into(),
                name: id.into(),
                kind,
            })
            .into(),
        weapons: vec![Weapon {
            id: "test-weapon".into(),
            name: "测试武器".into(),
            rarity: 4,
            stats: stats.map(|(id, _)| Some(id.into())),
        }],
    }
}

#[test]
fn visits_each_position_once_across_overlap_and_partial_tail() {
    let mut io = Inventory {
        offset: 211,
        clicks: vec![],
        stop_after: None,
        freeze_scroll: false,
    };
    let results = Results::default();
    workflow::scan(&mut io, &ScanSettings::default(), &catalog(), &results).unwrap();
    assert_eq!(
        io.clicks,
        (0..73).map(|i| (i / 9, i % 9)).collect::<Vec<_>>()
    );
    let items = results.0.lock().unwrap();
    assert_eq!(items.len(), 73);
    assert_eq!(items.last().unwrap().row, 9);
    assert_eq!(items.last().unwrap().column, 1);
    assert!(
        items
            .iter()
            .all(|item| item.evaluation.decision == essence::Decision::Keep)
    );
}

#[test]
fn stops_without_visiting_or_reporting_later_slots() {
    let mut io = Inventory {
        offset: 0,
        clicks: vec![],
        stop_after: Some(2),
        freeze_scroll: false,
    };
    let results = Results::default();
    let error =
        workflow::scan(&mut io, &ScanSettings::default(), &catalog(), &results).unwrap_err();
    assert!(error.downcast_ref::<AutomationStopped>().is_some());
    assert_eq!(io.clicks, vec![(0, 0), (0, 1)]);
    assert_eq!(results.0.lock().unwrap().len(), 2);
}

#[test]
fn failed_scroll_from_middle_does_not_claim_a_complete_inventory() {
    let mut io = Inventory {
        offset: 211,
        clicks: vec![],
        stop_after: None,
        freeze_scroll: true,
    };
    let results = Results::default();
    assert!(workflow::scan(&mut io, &ScanSettings::default(), &catalog(), &results).is_err());
    assert!(io.clicks.is_empty());
    assert!(results.0.lock().unwrap().is_empty());
}
