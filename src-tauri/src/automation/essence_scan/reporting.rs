use std::io::Cursor;

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::RgbaImage;

use crate::vision;

/// 结果只携带详情区域，避免为每份基质重复传输整个背包截图。
pub(super) fn screenshot_data_url(frame: &RgbaImage) -> Result<String> {
    let detail = vision::essence::detail_image(frame);
    let mut bytes = Cursor::new(Vec::new());
    detail.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(bytes.into_inner())
    ))
}
