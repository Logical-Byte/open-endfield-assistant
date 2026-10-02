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
        runtime::{FinishReason, Worker, WorkerExit},
    },
    data::AppData,
};

use super::{
    correction::{DEFAULT_CORRECTION_OVERRIDES, correct},
    reporting::{ScanReporter, encode_png_data_url},
};

pub(crate) struct SimulatedArchiveScanWorker {
    app_data: Arc<AppData>,
}

const SAMPLES: &[(&str, &str, &str)] = &[
    ("multi_media", "media", "受困者的录音"),
    ("text", "paper", "工团成员关于偷拍者的证词"),
    ("text", "paper", "工团大会预算申报宣讲草稿（第八"),
    ("text", "digital", "挂在竹子上的"),
    ("text", "digital", ""),
    ("text", "digital", "弩箭残片的记录"),
    ("text", "collection", "布龙泽的工具包"),
    ("document", "document", "泰拉"),
    ("document", "document", "中枢档案残片"),
    ("document", "report", "有关多桩裂地者遇袭事件的调查报告"),
];

impl SimulatedArchiveScanWorker {
    pub(crate) fn new(app_data: Arc<AppData>) -> Self {
        Self { app_data }
    }
}

impl Worker for SimulatedArchiveScanWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let reporter = ScanReporter::new(events);
        // 标题区域与前端的截图裁剪坐标一致。
        let image = encode_png_data_url(&RgbaImage::from_fn(1280, 720, |x, y| {
            if (360..876).contains(&x) && (48..134).contains(&y) {
                Rgba([218, 172, 86, 255])
            } else if (360..1080).contains(&x) && (180..620).contains(&y) {
                Rgba([58, 64, 74, 255])
            } else {
                Rgba([30, 34, 42, 255])
            }
        }));

        let finish_at = Instant::now() + Duration::from_secs(20);
        let mut sequence = 0;
        loop {
            let interval = Duration::from_millis(fastrand::u64(350..=650));
            let next_result_at = (Instant::now() + interval).min(finish_at);
            // 分片等待，保持停止操作的响应速度。
            while Instant::now() < next_result_at {
                if is_stop_requested(&stop) {
                    return WorkerExit::without_capture(FinishReason::Stopped);
                }
                thread::sleep(
                    next_result_at
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(100)),
                );
            }
            if is_stop_requested(&stop) {
                return WorkerExit::without_capture(FinishReason::Stopped);
            }
            if Instant::now() >= finish_at {
                break;
            }

            // 先展示固定样例，后续随机组合，让每次扫描都能覆盖不同的卡片状态。
            let (category, sub_category, title) = SAMPLES
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
            sequence += 1;

            let corrected = correct(
                self.app_data.archive_titles(),
                sub_category,
                &ocr_result,
                Some(DEFAULT_CORRECTION_OVERRIDES),
            );
            let status = if ocr_result.is_empty() {
                "failed"
            } else if corrected.is_some() {
                "success"
            } else {
                "unrecognized"
            };
            tracing::debug!(sequence, status, "上报模拟档案扫描结果");
            reporter.report(
                status,
                category,
                sub_category,
                image.clone(),
                ocr_result,
                corrected,
            );
        }

        WorkerExit::without_capture(FinishReason::Completed)
    }
}
