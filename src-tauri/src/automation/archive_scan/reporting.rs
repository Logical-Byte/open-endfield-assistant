//! 档案扫描记录的数据结构与上报器。
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

/// 对单个档案详情页面进行一次识别产生的一条扫描记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) struct ScannedItem {
    /// 识别时的具体游戏语言，人工纠正和 UI 语言变化保留此语境。
    pub game_locale: GameLocale,
    /// 扫描时所在的档案页面 ID（`pageType`：`multi_media` / `text` / `document`）。
    pub found_in_page: archive::Page,
    /// 扫描时所在的档案分类 ID（`categoryId`）。
    pub found_in_category: archive::Category,
    /// 档案详情页面截图（base64 PNG data URL，已缩小以控制事件体积）
    pub image: String,
    /// 原始 OCR 识别结果（人工纠错时保留）
    pub ocr_result: String,
    /// 正文区域的原始 OCR 文本，保留空白与标点，独立于用于判别的规范化文本。
    /// 仅在已知同名组请求正文且取得识别文本时填写。未识别正文、未检测到文字或
    /// 识别失败时省略，空字符串表示 OCR 已返回文本但内容为空。
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub body_ocr_result: Option<String>,
    /// 后端自动结果，人工决定由前端独立保存。
    pub matching: super::matching::MatchResult,
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

/// 扫描记录上报器：扫描工作流 → 自动化事件观察出口。
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

    /// 上报一条扫描记录。
    pub(super) fn report(
        &self,
        found_in_page: archive::Page,
        found_in_category: archive::Category,
        screenshot: &RgbaImage,
        ocr_result: String,
        body_ocr_result: Option<String>,
        matching: super::matching::MatchResult,
    ) {
        self.events.publish(Event::ArchiveItemScanned(ScannedItem {
            game_locale: GameLocale::ZhCn,
            found_in_page,
            found_in_category,
            image: encode_png_data_url(screenshot),
            ocr_result,
            body_ocr_result,
            matching,
        }));
    }
}
