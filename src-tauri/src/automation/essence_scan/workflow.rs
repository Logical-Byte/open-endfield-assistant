//! 基质背包扫描：回到页首，按位置遍历，观测滚动位移后继续扫描新行。

use std::{io::Cursor, time::Duration};

use anyhow::{Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{RgbaImage, imageops};

use crate::{
    automation::{Clock, Drag, Event, EventSink, Input, Point720p, ScreenCapture},
    essence::{self, Catalog, ScanSettings},
};

use super::{
    ScannedItem, layout, pagination,
    recognition::{self, Recognizer},
};

pub(super) fn scan(
    io: &mut (impl ScreenCapture + Input + Drag + Clock),
    settings: &ScanSettings,
    events: &dyn EventSink,
) -> Result<()> {
    let recognizer = Recognizer::new()?;
    let mut frame = reset_to_top(io, &recognizer)?;
    let mut offset = 0;
    let mut next_row = 0;
    let mut sequence = 0;
    for page in 1..=500 {
        recognizer.check_scene(&frame)?;
        // 首次截图确定本视口的占用情况，点击后的选中边框不会改变空槽判断。
        while pagination::row_y(next_row, offset) <= 588 {
            let y = pagination::row_y(next_row, offset);
            ensure!(y >= 125, "翻页越过了未扫描行，已停止扫描，请回到页首后重试");
            for (column, x) in layout::COLUMNS.into_iter().enumerate() {
                if !recognition::occupied(&frame, x, y as u32) {
                    continue;
                }
                io.click(Point720p { x, y: y as u32 })?;
                io.sleep(Duration::from_millis(300));
                let detail = io.screenshot()?;
                let essence = recognizer.recognize(&detail)?;
                let evaluation = essence::evaluate(&essence, settings, Catalog::bundled());
                sequence += 1;
                events.publish(Event::EssenceItemScanned(ScannedItem {
                    sequence,
                    page,
                    row: next_row + 1,
                    column: column as u32 + 1,
                    essence,
                    evaluation,
                    image: Some(detail_image(&detail)?),
                }));
            }
            next_row += 1;
        }

        let before = io.screenshot()?;
        recognizer.check_scene(&before)?;
        if recognition::scrollbar_at(&before, layout::SCROLL_BOTTOM) {
            return Ok(());
        }
        io.drag(layout::DRAG_START, layout::DRAG_END)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        recognizer.check_scene(&after)?;
        if pagination::unchanged(&before, &after) {
            // 不超过一页时游戏可能没有滚动条。可滚动列表未到底却无法推进时保留部分结果。
            if !recognition::scrollbar_visible(&after)
                || recognition::scrollbar_at(&after, layout::SCROLL_BOTTOM)
            {
                return Ok(());
            }
            bail!("背包没有继续滚动，无法确认已到末尾。已保留扫描结果，请检查游戏界面后重试");
        }
        let distance = pagination::measure_scroll(&before, &after)
            .filter(|distance| *distance > 0)
            .ok_or_else(|| {
                anyhow::anyhow!("无法确认翻页后的物品位置，已保留扫描结果，请回到页首后重试")
            })?;
        offset += distance;
        frame = after;
    }
    bail!("扫描达到 500 次翻页仍未到末尾，已停止并保留结果")
}

fn reset_to_top(
    io: &mut (impl ScreenCapture + Input + Drag + Clock),
    recognizer: &Recognizer,
) -> Result<RgbaImage> {
    let mut before = io.screenshot()?;
    recognizer.check_scene(&before)?;
    for _ in 0..500 {
        if recognition::scrollbar_at(&before, layout::SCROLL_TOP) {
            return Ok(before);
        }
        // 在卡片区域向下拖动，使内容回到顶部。每次等待后重新观测，不累计鼠标距离。
        io.drag(Point720p { x: 500, y: 160 }, layout::DRAG_START)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        recognizer.check_scene(&after)?;
        if pagination::unchanged(&before, &after) {
            ensure!(
                !recognition::scrollbar_visible(&after)
                    || recognition::scrollbar_at(&after, layout::SCROLL_TOP),
                "未能将基质列表滚动到顶部，请先手动回到页首"
            );
            return Ok(after);
        }
        before = after;
    }
    bail!("未能将基质列表滚动到顶部，请先手动回到页首")
}

fn detail_image(image: &RgbaImage) -> Result<String> {
    let roi = layout::DETAIL;
    let crop = imageops::crop_imm(image, roi.x0(), roi.y0(), roi.width(), roi.height()).to_image();
    let mut bytes = Cursor::new(Vec::new());
    crop.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(bytes.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use image::Rgba;

    use super::*;
    use crate::automation::{AutomationStopped, Key};

    #[derive(Default)]
    struct Results(Mutex<Vec<ScannedItem>>);

    impl EventSink for Results {
        fn publish(&self, event: Event) {
            if let Event::EssenceItemScanned(item) = event {
                self.0.lock().unwrap().push(item);
            }
        }
    }

    /// 73 枚基质，9 行，尾页仅剩一格。所有详情属性相同，不能按属性去重。
    struct Inventory {
        offset: u32,
        clicks: Vec<(u32, u32)>,
        stop_after: Option<usize>,
        freeze_scroll: bool,
    }

    impl ScreenCapture for Inventory {
        fn screenshot(&mut self) -> Result<RgbaImage> {
            let mut frame = RgbaImage::from_pixel(1280, 720, Rgba([10, 10, 10, 255]));
            for y in layout::GRID.y0()..layout::GRID.y1() {
                let world_y = y + self.offset;
                let row =
                    ((f64::from(world_y) - layout::FIRST_ROW_Y) / layout::ROW_PITCH).round() as i32;
                if !(0..9).contains(&row) {
                    continue;
                }
                let center = pagination::row_y(row as u32, 0);
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
            // 用真实内置模板拼详情，验证点击、识别、纯判断和上报组成的完整工作流。
            for (name, roi) in [
                ("scene", layout::SCENE),
                ("unlocked", layout::LOCK),
                ("unabandoned", layout::ABANDON),
                ("gat_passive_attr_str", layout::STATS[0]),
                ("gat_passive_attr_atk", layout::STATS[1]),
                ("gst_passive_tactic", layout::STATS[2]),
            ] {
                let (_, bytes) = super::super::templates::TEMPLATES
                    .iter()
                    .find(|(id, _)| *id == name)
                    .unwrap();
                let template = image::load_from_memory(bytes)?.to_rgba8();
                imageops::replace(
                    &mut frame,
                    &template,
                    i64::from(roi.x0()),
                    i64::from(roi.y0()),
                );
            }
            for &y in &layout::LEVEL_Y {
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

    #[test]
    fn visits_each_position_once_across_overlap_and_partial_tail() {
        let mut io = Inventory {
            offset: 211,
            clicks: vec![],
            stop_after: None,
            freeze_scroll: false,
        };
        let results = Results::default();
        scan(&mut io, &ScanSettings::default(), &results).unwrap();
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
        let error = scan(&mut io, &ScanSettings::default(), &results).unwrap_err();
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
        assert!(scan(&mut io, &ScanSettings::default(), &results).is_err());
        assert!(io.clicks.is_empty());
        assert!(results.0.lock().unwrap().is_empty());
    }
}
