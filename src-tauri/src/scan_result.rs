//! 扫描结果数据结构与序列化辅助。
//!
//! 每次识别一份档案后，后端把 [`ScanResult`] 通过 Tauri 事件 `scan-result`
//! 推送给前端，前端以卡片形式直观展示（含详情页截图与可编辑的 OCR 文本）。

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, RgbaImage, imageops};
use serde::Serialize;

/// 单份档案的扫描结果。
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    /// 识别状态：`success`（OCR 结果非空）或 `failed`（OCR 结果为空）
    pub status: String,
    /// 全局序号（从 1 开始，跨分类连续递增）
    pub index: u32,
    /// 档案库分类：音像存档 / 见闻辑录 / 中枢档案（单次扫描时为"未知"）
    pub category: String,
    /// 档案详情页面截图（base64 PNG data URL，已缩小以控制事件体积）
    pub image: String,
    /// OCR 识别结果（前端可编辑）
    pub ocr_result: String,
}

/// 截图编码为 data URL 前的最大宽度（等比缩小，控制事件体积与内存占用）。
const MAX_IMAGE_WIDTH: u32 = 640;

/// 把 720p 截图编码为 base64 PNG data URL（供前端 `<img>` 直接显示）。
///
/// 截图会按最大宽度等比缩小，在保证可读性的同时控制事件体积。
pub fn encode_png_data_url(img: &RgbaImage) -> String {
    // 等比缩小，控制事件体积
    let scaled = if img.width() > MAX_IMAGE_WIDTH {
        let h = ((img.height() as u64 * MAX_IMAGE_WIDTH as u64) / img.width() as u64) as u32;
        imageops::resize(img, MAX_IMAGE_WIDTH, h, imageops::FilterType::Triangle)
    } else {
        img.clone()
    };

    let mut buf: Vec<u8> = Vec::new();
    let dyn_img = DynamicImage::ImageRgba8(scaled).to_rgb8();
    dyn_img
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("PNG 编码失败");

    format!("data:image/png;base64,{}", STANDARD.encode(&buf))
}
