//! 档案库扫描结果的数据结构与上报器。
//!
//! `ScannedItem` 是档案扫描的逐条产出。`super::Worker` 把应用层注入的
//! [`EventSink`] 包装为 `ScanReporter`，工作流不依赖 Tauri 句柄。

use std::io::Cursor;
use std::sync::Arc;

use crate::{data::archive, locale::GameLocale};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, RgbaImage, imageops};
use serde::Serialize;
use ts_rs::TS;

use crate::automation::{Event, EventSink};

/// 单份扫描的识别状态，不表示自动化任务本身是否成功。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) enum ScannedItemStatus {
    /// 已匹配到档案候选。
    Success,
    /// 有 OCR 文本，但未匹配到可靠候选。
    Unrecognized,
    /// OCR 文本为空或识别失败。
    Failed,
}

/// 单份档案的扫描结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) struct ScannedItem {
    /// 识别状态：`success`（纠错成功）/ `unrecognized`（识别到文本但无法纠错）/
    /// `failed`（OCR 结果为空）
    pub status: ScannedItemStatus,
    /// 识别时的具体游戏语言，人工纠正和 UI 语言变化保留此语境。
    pub game_locale: GameLocale,
    /// 扫描时所在的档案库大类 id（pageType：multi_media / text / document）
    pub found_in_page: archive::Page,
    /// 扫描时所在的档案库小类 id（categoryId）
    pub found_in_category: archive::Category,
    /// 档案详情页面截图（base64 PNG data URL，已缩小以控制事件体积）
    pub image: String,
    /// 原始 OCR 识别结果（人工纠错时保留）
    pub ocr_result: String,
    /// 纠错后的档案标题（无法识别时为 `None`）
    pub corrected_title: Option<String>,
    /// 纠错命中的档案 id（allItems 的 id，当前小分类下同标题多条时返回全部）
    pub corrected_match_item_ids: Vec<archive::ArchiveId>,
}

/// 截图编码为 data URL 前的最大宽度（等比缩小，控制事件体积与内存占用）。
const MAX_IMAGE_WIDTH: u32 = 1280;

/// 把 720p 截图编码为 base64 PNG data URL（供前端 `<img>` 直接显示）。
///
/// 截图会按最大宽度等比缩小，在保证可读性的同时控制事件体积。
fn encode_png_data_url(img: &RgbaImage) -> String {
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

/// 扫描结果上报器：扫描工作流 → 自动化事件观察出口。
///
/// `super::Worker` 创建 `ScanReporter` 并注入扫描工作流。
/// 工作流只负责发布完整的领域结果，不关心观察者如何把事件传递给前端。
pub(super) struct ScanReporter {
    events: Arc<dyn EventSink>,
}

impl ScanReporter {
    /// 创建上报器。
    pub(super) fn new(events: Arc<dyn EventSink>) -> Self {
        Self { events }
    }

    /// 上报一份扫描结果。
    pub(super) fn report(
        &self,
        found_in_page: archive::Page,
        found_in_category: archive::Category,
        screenshot: &RgbaImage,
        ocr_result: String,
        corrected: Option<super::ocr_correction::Corrected>,
    ) {
        let status = if ocr_result.is_empty() {
            ScannedItemStatus::Failed
        } else if corrected.is_some() {
            ScannedItemStatus::Success
        } else {
            ScannedItemStatus::Unrecognized
        };
        let (corrected_title, corrected_match_item_ids) = match corrected {
            Some(c) => (Some(c.title), c.item_ids),
            None => (None, Vec::new()),
        };
        self.events.publish(Event::ArchiveItemScanned(ScannedItem {
            status,
            game_locale: GameLocale::ZhCn,
            found_in_page,
            found_in_category,
            image: encode_png_data_url(screenshot),
            ocr_result,
            corrected_title,
            corrected_match_item_ids,
        }));
    }
}
