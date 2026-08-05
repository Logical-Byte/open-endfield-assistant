//! 档案库扫描结果的数据结构与上报器。
//!
//! `ScanResult` 与游戏强相关（档案扫描的产出），故归入档案库扫描任务模块，
//! 不属于基础设施层。上报机制（通道 + 序号）由应用层（`crate::controller`）
//! 创建并注入，保持依赖方向"应用 → 领域"。

use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, RgbaImage, imageops};
use serde::Serialize;

/// 单份档案的扫描结果。
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    /// 识别状态：`success`（OCR 结果非空）或 `failed`（OCR 结果为空）
    pub status: String,
    /// 全局序号（从 1 开始，跨主任务 / 单次扫描连续递增）
    pub index: u32,
    /// 档案库分类：音像存档 / 见闻辑录 / 中枢档案（单次扫描时为"未知"）
    pub category: String,
    /// 档案详情页面截图（base64 PNG data URL，已缩小以控制事件体积）
    pub image: String,
    /// OCR 识别结果（前端可编辑）
    pub ocr_result: String,
}

/// 截图编码为 data URL 前的最大宽度（等比缩小，控制事件体积与内存占用）。
const MAX_IMAGE_WIDTH: u32 = 1280;

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

/// 扫描结果上报器：任务层 → 前端事件通道。
///
/// 持有通道发送端与全局序号（跨主任务 / 单次扫描连续递增）；
/// 由 [`crate::controller::Controller`] 创建并注入任务函数。
/// 任务只负责"报了什么结果"，不关心通道如何到达前端。
#[derive(Clone)]
pub struct ScanReporter {
    tx: mpsc::Sender<ScanResult>,
    index: Arc<AtomicU32>,
}

impl ScanReporter {
    /// 创建上报器。
    pub fn new(tx: mpsc::Sender<ScanResult>, index: Arc<AtomicU32>) -> Self {
        Self { tx, index }
    }

    /// 上报一份扫描结果（序号自动递增，从 1 开始）。
    pub fn report(&self, status: &str, category: &str, image: String, ocr_result: String) {
        let index = self.index.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.tx.send(ScanResult {
            status: status.to_string(),
            index,
            category: category.to_string(),
            image,
            ocr_result,
        });
    }
}
