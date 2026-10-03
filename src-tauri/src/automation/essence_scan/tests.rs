use std::{sync::Mutex, time::Duration};

use anyhow::Result;
use image::{Rgba, RgbaImage};

use crate::{
    automation::{
        AutomationStopped, Clock, Drag, Event, EventSink, Input, Key, Point720p, ScreenCapture,
        TemplateMatch, TemplateMatching, TemplateTarget,
    },
    essence::{self, Catalog, MarkAction, ScanSettings, Stat, StatKind, Weapon},
};

use super::{Marking, ScannedItem, layout, workflow};

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
#[derive(Default)]
struct Inventory {
    offset: u32,
    clicks: Vec<(u32, u32)>,
    stop_after: Option<usize>,
    freeze_scroll: bool,
    marks: Vec<MarkAction>,
    mark_effect: MarkEffect,
    locked: bool,
    abandoned: bool,
    changed_identity: bool,
}

#[derive(Default)]
enum MarkEffect {
    #[default]
    Applied,
    NoEffect,
    ChangedIdentity,
    StopAfterClick,
}

impl ScreenCapture for Inventory {
    fn screenshot(&mut self) -> Result<RgbaImage> {
        if matches!(self.mark_effect, MarkEffect::StopAfterClick) && !self.marks.is_empty() {
            return Err(AutomationStopped.into());
        }
        let mut frame = RgbaImage::from_pixel(1280, 720, Rgba([10, 10, 10, 255]));
        // 匹配器从传入帧读取状态，不能偷读截图之后的 Inventory 状态。
        frame.put_pixel(
            0,
            0,
            Rgba([
                u8::from(self.locked),
                u8::from(self.abandoned),
                u8::from(self.changed_identity),
                255,
            ]),
        );
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
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>> {
        let name = target
            .template_name
            .rsplit('/')
            .next()
            .unwrap()
            .trim_end_matches(".png");
        let [locked, abandoned, changed_identity, _] = screenshot.get_pixel(0, 0).0;
        let matched = name == "scene"
            || (name == "locked" && locked == 1)
            || (name == "unlocked" && locked == 0)
            || (name == "abandoned" && abandoned == 1)
            || (name == "unabandoned" && abandoned == 0)
            || (name == "gat_passive_attr_str" && target.roi.y0() == 235 && changed_identity == 0)
            || (name == "gat_passive_attr_agi" && target.roi.y0() == 235 && changed_identity == 1)
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
        let action = match (point.x, point.y) {
            (1226, 191) => Some(MarkAction::Lock),
            (1205, 189) => Some(MarkAction::Abandon),
            _ => None,
        };
        if let Some(action) = action {
            self.marks.push(action);
            if !matches!(self.mark_effect, MarkEffect::NoEffect) {
                match action {
                    MarkAction::Lock => self.locked = true,
                    MarkAction::Abandon => self.abandoned = true,
                }
            }
            self.changed_identity = matches!(self.mark_effect, MarkEffect::ChangedIdentity);
            return Ok(());
        }
        if self.stop_after == Some(self.clicks.len()) {
            return Err(AutomationStopped.into());
        }
        self.locked = false;
        self.abandoned = false;
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
        ..Inventory::default()
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
    assert!(io.marks.is_empty());
    assert!(
        items
            .iter()
            .all(|item| item.evaluation.decision == essence::Decision::Keep)
    );
}

#[test]
fn stops_without_visiting_or_reporting_later_slots() {
    let mut io = Inventory {
        stop_after: Some(2),
        ..Inventory::default()
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
        freeze_scroll: true,
        ..Inventory::default()
    };
    let results = Results::default();
    assert!(workflow::scan(&mut io, &ScanSettings::default(), &catalog(), &results).is_err());
    assert!(io.clicks.is_empty());
    assert!(results.0.lock().unwrap().is_empty());
}

#[test]
fn confirms_each_lock_or_abandon_from_a_new_frame() {
    for action in [MarkAction::Lock, MarkAction::Abandon] {
        let mut io = Inventory {
            stop_after: Some(2),
            ..Inventory::default()
        };
        let settings = ScanSettings {
            auto_mark: true,
            excluded_weapon_ids: if action == MarkAction::Abandon {
                vec!["test-weapon".into()]
            } else {
                vec![]
            },
            ..ScanSettings::default()
        };
        let results = Results::default();
        let error = workflow::scan(&mut io, &settings, &catalog(), &results).unwrap_err();
        assert!(error.downcast_ref::<AutomationStopped>().is_some());
        assert_eq!(io.marks, vec![action, action]);
        let items = results.0.lock().unwrap();
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|item| item.marking == Marking::Applied { action })
        );
        assert!(items.iter().all(
            |item| item.essence.locked == Some(false) && item.essence.abandoned == Some(false)
        ));
    }
}

#[test]
fn reports_failed_marking_before_stopping_without_retrying_or_visiting_next_slot() {
    for effect in [MarkEffect::NoEffect, MarkEffect::ChangedIdentity] {
        let mut io = Inventory {
            mark_effect: effect,
            ..Inventory::default()
        };
        let settings = ScanSettings {
            auto_mark: true,
            ..ScanSettings::default()
        };
        let results = Results::default();
        assert!(workflow::scan(&mut io, &settings, &catalog(), &results).is_err());
        assert_eq!(io.clicks, vec![(0, 0)]);
        assert_eq!(io.marks, vec![MarkAction::Lock]);
        let items = results.0.lock().unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0].marking,
            Marking::Failed {
                action: MarkAction::Lock,
                ..
            }
        ));
    }
}

#[test]
fn stopping_after_a_mark_click_keeps_the_unconfirmed_item_visible_and_preserves_stop() {
    let mut io = Inventory {
        mark_effect: MarkEffect::StopAfterClick,
        ..Inventory::default()
    };
    let settings = ScanSettings {
        auto_mark: true,
        ..ScanSettings::default()
    };
    let results = Results::default();
    let error = workflow::scan(&mut io, &settings, &catalog(), &results).unwrap_err();
    assert!(error.downcast_ref::<AutomationStopped>().is_some());
    assert_eq!(io.clicks, vec![(0, 0)]);
    assert_eq!(io.marks, vec![MarkAction::Lock]);
    let items = results.0.lock().unwrap();
    assert_eq!(items.len(), 1);
    let Marking::Failed { action, error } = &items[0].marking else {
        panic!("未保留标记未确认结果")
    };
    assert_eq!(*action, MarkAction::Lock);
    assert!(error.contains("尚未确认"));
}
