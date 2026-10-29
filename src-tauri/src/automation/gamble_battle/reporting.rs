//! 真实与模拟工作者共用的命中事件上报。

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, RgbaImage};

use super::{
    Progress,
    model::{self, Card, MatchedHand, StopReason},
};
use crate::automation::{self, Event, EventSink};

pub(super) fn report_match(
    events: &dyn EventSink,
    cards: [Card; 5],
    reasons: Vec<StopReason>,
    image: RgbaImage,
    progress: Progress,
) -> Result<(), automation::WorkerError> {
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| automation::WorkerError::Custom {
            message: format!("保存命中截图失败：{error}"),
        })?;
    events.publish(Event::GambleBattleMatched(MatchedHand {
        round: progress.round,
        elapsed_seconds: progress.elapsed_seconds,
        cards,
        hand: model::classify(&cards),
        reasons,
        image: format!(
            "data:image/png;base64,{}",
            STANDARD.encode(png.into_inner())
        ),
    }));
    Ok(())
}
