//! 基质背包扫描：回到页首，按位置遍历，观测滚动位移后继续扫描新行。

use std::time::Duration;

use anyhow::{Result, bail, ensure};
use image::RgbaImage;

use crate::{
    automation::{
        Clock, Drag, Event, EventSink, Input, Point720p, ScreenCapture, TemplateMatching,
    },
    essence::{self, Catalog, ScanSettings},
    navigation, vision,
};

use super::{ScannedItem, layout, marking, reporting};

pub(super) fn scan(
    io: &mut (impl ScreenCapture + Input + Drag + Clock + TemplateMatching),
    settings: &ScanSettings,
    catalog: &Catalog,
    events: &dyn EventSink,
) -> Result<()> {
    let mut frame = reset_to_top(io)?;
    let mut offset = 0;
    let mut next_row = 0;
    let mut sequence = 0;
    for page in 1..=500 {
        navigation::require_essence_inventory(io, &frame)?;
        // 首次截图确定本视口的占用情况，点击后的选中边框不会改变空槽判断。
        while layout::row_y(next_row, offset) <= 588 {
            let y = layout::row_y(next_row, offset);
            ensure!(y >= 125, "翻页越过了未扫描行，已停止扫描，请回到页首后重试");
            for (column, x) in layout::COLUMNS.into_iter().enumerate() {
                if !vision::essence::occupied(&frame, x, y as u32) {
                    continue;
                }
                io.click(Point720p { x, y: y as u32 })?;
                io.sleep(Duration::from_millis(300));
                let detail = io.screenshot()?;
                navigation::require_essence_inventory(io, &detail)?;
                let essence = vision::essence::recognize(io, &detail)?;
                let evaluation = essence::evaluate(&essence, settings, catalog);
                let image = Some(reporting::screenshot_data_url(&detail)?);
                let plan = essence::plan_marking(&essence, evaluation.decision, settings.auto_mark);
                let (marking, mark_result) = marking::apply(io, &essence, plan);
                sequence += 1;
                events.publish(Event::EssenceItemScanned(ScannedItem {
                    sequence,
                    page,
                    row: next_row + 1,
                    column: column as u32 + 1,
                    essence,
                    evaluation,
                    marking,
                    image,
                }));
                mark_result?;
            }
            next_row += 1;
        }

        let before = io.screenshot()?;
        navigation::require_essence_inventory(io, &before)?;
        if vision::essence::scrollbar_at(&before, layout::SCROLL_BOTTOM) {
            return Ok(());
        }
        io.drag(layout::DRAG_START, layout::DRAG_END)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        navigation::require_essence_inventory(io, &after)?;
        if frames_unchanged(&before, &after) {
            // 不超过一页时游戏可能没有滚动条。可滚动列表未到底却无法推进时保留部分结果。
            if !vision::essence::scrollbar_visible(&after)
                || vision::essence::scrollbar_at(&after, layout::SCROLL_BOTTOM)
            {
                return Ok(());
            }
            bail!("背包没有继续滚动，无法确认已到末尾。已保留扫描结果，请检查游戏界面后重试");
        }
        let distance =
            vision::scroll::measure_vertical_shift(&before, &after, layout::SCROLL_CONTENT, 375)
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
    io: &mut (impl ScreenCapture + Input + Drag + Clock + TemplateMatching),
) -> Result<RgbaImage> {
    let mut before = io.screenshot()?;
    navigation::require_essence_inventory(io, &before)?;
    for _ in 0..500 {
        if vision::essence::scrollbar_at(&before, layout::SCROLL_TOP) {
            return Ok(before);
        }
        // 在卡片区域向下拖动，使内容回到顶部。每次等待后重新观测，不累计鼠标距离。
        io.drag(Point720p { x: 500, y: 160 }, layout::DRAG_START)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        navigation::require_essence_inventory(io, &after)?;
        if frames_unchanged(&before, &after) {
            ensure!(
                !vision::essence::scrollbar_visible(&after)
                    || vision::essence::scrollbar_at(&after, layout::SCROLL_TOP),
                "未能将基质列表滚动到顶部，请先手动回到页首"
            );
            return Ok(after);
        }
        before = after;
    }
    bail!("未能将基质列表滚动到顶部，请先手动回到页首")
}

fn frames_unchanged(before: &RgbaImage, after: &RgbaImage) -> bool {
    vision::scroll::unchanged(before, after, layout::SCROLL_CONTENT)
        && vision::essence::scrollbar_unchanged(before, after)
}
