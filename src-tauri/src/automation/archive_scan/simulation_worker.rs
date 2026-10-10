//! 无需游戏窗口的档案扫描，供前端调试使用。

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use image::{Rgba, RgbaImage};

use crate::{
    automation::{
        EventSink, StopToken, is_stop_requested,
        runtime::{self, FinishReason, WorkerExit},
    },
    data::{AppData, archive},
};

use super::{
    ocr_correction::{DEFAULT_CORRECTION_OVERRIDES, match_with_correction},
    reporting::ScanReporter,
};

pub(crate) struct SimulationWorker {
    app_data: Arc<AppData>,
}

const SAMPLES: &[(archive::Page, archive::Category, &str)] = &[
    (
        archive::Page::MultiMedia,
        archive::Category::Media,
        "受困者的录音",
    ),
    (
        archive::Page::Text,
        archive::Category::Paper,
        "工团成员关于偷拍者的证词",
    ),
    (
        archive::Page::Text,
        archive::Category::Paper,
        "工团大会预算申报宣讲草稿（第八",
    ),
    (
        archive::Page::Text,
        archive::Category::Digital,
        "挂在竹子上的",
    ),
    (archive::Page::Text, archive::Category::Digital, ""),
    (
        archive::Page::Text,
        archive::Category::Digital,
        "弩箭残片的记录",
    ),
    (
        archive::Page::Text,
        archive::Category::Collection,
        "布龙泽的工具包",
    ),
    (archive::Page::Document, archive::Category::Document, "泰拉"),
    (
        archive::Page::Document,
        archive::Category::Document,
        "中枢档案残片",
    ),
    (
        archive::Page::Document,
        archive::Category::Report,
        "有关多桩裂地者遇袭事件的调查报告",
    ),
];

impl SimulationWorker {
    pub(crate) fn new(app_data: Arc<AppData>) -> Self {
        Self { app_data }
    }
}

impl runtime::Worker for SimulationWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let reporter = ScanReporter::new(events);
        let image = sample_image();

        let finish_at = Instant::now() + Duration::from_secs(20);
        let mut sequence = 0;
        let finish_reason = loop {
            let interval = Duration::from_millis(fastrand::u64(350..=650));
            let next_result_at = (Instant::now() + interval).min(finish_at);
            if !wait_until_or_stopped(next_result_at, &stop) {
                break FinishReason::Stopped;
            }
            if Instant::now() >= finish_at {
                break FinishReason::Completed;
            }

            let (page, category, ocr_result) = sample_ocr_result(sequence);
            sequence += 1;

            let corrected = match_with_correction(
                self.app_data.archives(),
                category,
                &ocr_result,
                Some(&DEFAULT_CORRECTION_OVERRIDES),
            );
            tracing::debug!(sequence, "上报模拟档案扫描结果");
            reporter.report(page, category, &image, ocr_result, corrected);
        };

        if matches!(finish_reason, FinishReason::Stopped) {
            // 模拟一秒收尾，让前端有时间展示“正在停止扫描”，期间不再产出结果。
            thread::sleep(Duration::from_secs(1));
        }
        WorkerExit {
            reason: finish_reason,
            capture: None,
        }
    }
}

/// 分片等待，保持停止操作的响应速度。到达时刻后仍优先检查停止请求。
fn wait_until_or_stopped(deadline: Instant, stop: &StopToken) -> bool {
    while Instant::now() < deadline {
        if is_stop_requested(stop) {
            return false;
        }
        thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(100)),
        );
    }
    !is_stop_requested(stop)
}

fn sample_ocr_result(sequence: usize) -> (archive::Page, archive::Category, String) {
    // 先展示固定样例，后续随机组合，让每次扫描都能覆盖不同的卡片状态。
    let (page, category, title) = SAMPLES
        .get(sequence)
        .copied()
        .unwrap_or_else(|| SAMPLES[fastrand::usize(..SAMPLES.len())]);
    let ocr_result = if sequence < SAMPLES.len() {
        title.to_string()
    } else {
        match fastrand::u8(0..3) {
            0 => title.to_string(),
            1 => String::new(),
            _ => format!("模拟档案 {}：{title}", sequence + 1),
        }
    };
    (page, category, ocr_result)
}

fn sample_image() -> RgbaImage {
    // 标题区域与前端的截图裁剪坐标一致。
    RgbaImage::from_fn(1280, 720, |x, y| {
        if (360..876).contains(&x) && (48..134).contains(&y) {
            Rgba([218, 172, 86, 255])
        } else if (360..1080).contains(&x) && (180..620).contains(&y) {
            Rgba([58, 64, 74, 255])
        } else {
            Rgba([30, 34, 42, 255])
        }
    })
}
