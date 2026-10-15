use image::RgbaImage;
use std::time::Duration;

use super::{
    layout,
    model::Phase,
    recognition::{self, failure},
    reporting::Reporter,
};
use crate::{
    automation::{self, Clock, Drag, Input, ScreenCapture, TemplateMatching},
    data::essence,
    vision::scroll,
};

pub(super) fn scan(
    io: &mut (impl ScreenCapture + Input + Drag + Clock + TemplateMatching),
    catalog: &essence::Catalog,
    reporter: &Reporter,
) -> Result<(), automation::WorkerError> {
    io.move_mouse_to_safe_position()?;
    let mut frame = reset_to_top(io)?;
    let mut offset = 0;
    let mut next_row = 0;
    // OEA 有界防挂起策略，不作为列表末尾判断。每次仅推进三行，500 次远超正常背包容量。
    for _ in 0..500 {
        recognition::require_inventory(io, &frame)?;
        reporter.phase(Phase::Scanning);
        while layout::row_y(next_row, offset) <= layout::MAX_ROW_Y {
            let y = layout::row_y(next_row, offset);
            if y < layout::MIN_ROW_Y {
                return Err(failure(
                    "翻页越过了未扫描行，已保留部分结果，请回到页首重试",
                ));
            }
            for (column, x) in layout::COLUMNS.into_iter().enumerate() {
                if !recognition::occupied(&frame, x, y as u32) {
                    continue;
                }
                io.click(automation::Point720p { x, y: y as u32 })?;
                // EER engine.py 单枚点击后等待 0.3 秒。
                io.sleep(Duration::from_millis(300));
                let detail = io.screenshot()?;
                recognition::require_inventory(io, &detail)?;
                reporter.record(recognition::recognize(
                    io,
                    &detail,
                    catalog,
                    next_row + 1,
                    column as u32 + 1,
                )?);
            }
            next_row += 1;
        }
        let before = io.screenshot()?;
        recognition::require_inventory(io, &before)?;
        if recognition::scrollbar_at(&before, layout::SCROLL_BOTTOM) {
            return Ok(());
        }
        reporter.phase(Phase::Scrolling);
        io.drag(layout::DRAG_START, layout::DRAG_END)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        recognition::require_inventory(io, &after)?;
        if scroll::unchanged(&before, &after, layout::SCROLL_CONTENT) {
            if !recognition::scrollbar_visible(&after)
                || recognition::scrollbar_at(&after, layout::SCROLL_BOTTOM)
            {
                return Ok(());
            }
            return Err(failure(
                "基质列表未继续滚动，无法确认已到末尾，已保留部分结果",
            ));
        }
        let distance = scroll::measure_vertical_shift(&before, &after, layout::SCROLL_CONTENT, 375)
            .filter(|distance| *distance > 0)
            .ok_or_else(|| failure("无法确认翻页位移，已保留部分结果，请回到页首重试"))?;
        offset += distance;
        frame = after;
    }
    Err(failure("基质扫描达到翻页上限，已保留部分结果"))
}

fn reset_to_top(
    io: &mut (impl ScreenCapture + Input + Drag + Clock + TemplateMatching),
) -> Result<RgbaImage, automation::WorkerError> {
    let mut before = io.screenshot()?;
    recognition::require_inventory(io, &before)?;
    for _ in 0..500 {
        if recognition::scrollbar_at(&before, layout::SCROLL_TOP) {
            return Ok(before);
        }
        io.drag(layout::RESET_START, layout::DRAG_START)?;
        io.sleep(Duration::from_millis(350));
        let after = io.screenshot()?;
        recognition::require_inventory(io, &after)?;
        if scroll::unchanged(&before, &after, layout::SCROLL_CONTENT) {
            if !recognition::scrollbar_visible(&after)
                || recognition::scrollbar_at(&after, layout::SCROLL_TOP)
            {
                return Ok(after);
            }
            return Err(failure("未能确认基质列表已回到顶部，请手动回到页首重试"));
        }
        before = after;
    }
    Err(failure("基质列表回到顶部超时，请手动回到页首重试"))
}
