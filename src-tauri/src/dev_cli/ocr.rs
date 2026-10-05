//! 同一份 CLI 在每个后端分别产出报告，比较阶段只读取报告。
use std::{collections::BTreeMap, path::Path, time::Instant};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{args::Rect, commands::Output};
use crate::{automation, vision::ocr};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Settings {
    region: Option<Rect>,
    archive_title: bool,
    threads: u32,
    warmup: u32,
    repeat: u32,
}

#[derive(Serialize, Deserialize)]
struct Report {
    backend: String,
    settings: Settings,
    initialization_ms: f64,
    images: Vec<ImageResult>,
}

#[derive(Serialize, Deserialize)]
struct ImageResult {
    image: String,
    size: [u32; 2],
    recognition: Option<ocr::Recognition>,
    stable_text: bool,
    first_ms: f64,
    samples_ms: Vec<f64>,
    mean_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
}

pub(super) fn recognize(
    input: &Path,
    models: &Path,
    region: Option<Rect>,
    archive_title: bool,
    threads: u32,
    warmup: u32,
    repeat: u32,
) -> Result<Output> {
    let settings = Settings {
        region,
        archive_title,
        threads,
        warmup,
        repeat,
    };
    let mut paths = if input.is_dir() {
        std::fs::read_dir(input)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .filter(|path| {
                path.is_file()
                    && path.extension().is_some_and(|ext| {
                        ["png", "jpg", "jpeg", "bmp", "webp"]
                            .iter()
                            .any(|candidate| ext.eq_ignore_ascii_case(candidate))
                    })
            })
            .collect::<Vec<_>>()
    } else {
        vec![input.to_path_buf()]
    };
    paths.sort();
    ensure!(!paths.is_empty(), "输入目录没有可识别的图片");
    let started = Instant::now();
    let mut engine = ocr::OcrEngine::new(
        models,
        ocr::Config {
            threads: threads as usize,
        },
    )?;
    let initialization_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut images = Vec::new();
    for path in paths {
        let mut image = image::open(&path)
            .with_context(|| format!("读取图片 {}", path.display()))?
            .to_rgba8();
        if archive_title {
            image = automation::normalize_screenshot(image)?;
        }
        let size = [image.width(), image.height()];
        let region = if archive_title {
            Some(automation::archive_scan::OCR_ROI)
        } else {
            region
                .map(|rect| rect.validate(size[0], size[1]))
                .transpose()?
        };
        let rgb = image::DynamicImage::ImageRgba8(image.clone()).to_rgb8();
        let mut recognize = || {
            if let Some(region) = region {
                engine.recognize_region(&image, region)
            } else {
                engine.recognize(&rgb).map(Some)
            }
        };
        let start = Instant::now();
        let recognition = recognize()?;
        let first_ms = start.elapsed().as_secs_f64() * 1000.0;
        let text = recognition.as_ref().map(|result| result.text.as_str());
        let mut stable_text = true;
        let mut samples_ms = Vec::new();
        for iteration in 0..u64::from(warmup) + u64::from(repeat) {
            let start = Instant::now();
            let result = recognize()?;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            stable_text &= result.as_ref().map(|result| result.text.as_str()) == text;
            if iteration >= u64::from(warmup) {
                samples_ms.push(elapsed);
            }
        }
        let mean_ms = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;
        let mut ordered = samples_ms.clone();
        ordered.sort_by(f64::total_cmp);
        images.push(ImageResult {
            image: path
                .file_name()
                .context("图片没有文件名")?
                .to_string_lossy()
                .into_owned(),
            size,
            recognition,
            stable_text,
            first_ms,
            samples_ms,
            mean_ms,
            p50_ms: ordered[(ordered.len() - 1) / 2],
            p95_ms: ordered[(ordered.len() as f64 * 0.95).ceil() as usize - 1],
        });
    }
    let report = Report {
        backend: ocr::BACKEND_NAME.to_owned(),
        settings,
        initialization_ms,
        images,
    };
    let lines = report
        .images
        .iter()
        .map(|image| {
            format!(
                "{}: {:?} ({:.3} ms)",
                image.image,
                image.recognition.as_ref().map(|result| &result.text),
                image.mean_ms
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Output {
        human: format!("{}\n{lines}", report.backend),
        json: json!({"ok":true, "command":"ocr", "report":report}),
    })
}

pub(super) fn compare(before: &Path, after: &Path) -> Result<Output> {
    #[derive(Deserialize)]
    struct Envelope {
        report: Report,
    }
    let read = |path: &Path| -> Result<Report> {
        let bytes = std::fs::read(path).with_context(|| format!("读取报告 {}", path.display()))?;
        Ok(serde_json::from_slice::<Envelope>(&bytes)
            .with_context(|| format!("解析报告 {}", path.display()))?
            .report)
    };
    compare_reports(read(before)?, read(after)?)
}

fn compare_reports(before: Report, after: Report) -> Result<Output> {
    ensure!(
        before.settings == after.settings,
        "两个报告的区域、线程或计时配置不一致"
    );
    let index = |images: Vec<ImageResult>| -> Result<BTreeMap<String, ImageResult>> {
        let mut indexed = BTreeMap::new();
        for image in images {
            ensure!(
                !indexed.contains_key(&image.image),
                "报告包含重复图片名 {}",
                image.image
            );
            indexed.insert(image.image.clone(), image);
        }
        Ok(indexed)
    };
    let left = index(before.images)?;
    let right = index(after.images)?;
    ensure!(
        left.keys().eq(right.keys()) && !left.is_empty(),
        "两个报告的图片集合不一致或为空"
    );
    let mut rows = Vec::new();
    let mut changed = 0;
    for (name, a) in &left {
        let b = &right[name];
        ensure!(a.size == b.size, "图片 {name} 的尺寸不一致");
        let text =
            |image: &ImageResult| image.recognition.as_ref().map(|result| result.text.clone());
        let equal = text(a) == text(b);
        changed += usize::from(!equal);
        rows.push(json!({"image":name, "same_text":equal, "before":a.recognition, "after":b.recognition,
            "stable_text":a.stable_text && b.stable_text, "before_mean_ms":a.mean_ms, "after_mean_ms":b.mean_ms,
            "after_over_before":b.mean_ms / a.mean_ms}));
    }
    Ok(Output {
        human: format!(
            "{} → {}: {} images, {changed} changed texts",
            before.backend,
            after.backend,
            rows.len()
        ),
        json: json!({"ok":true,"command":"ocr-compare","before_backend":before.backend,"after_backend":after.backend,
            "changed_texts":changed,"images":rows}),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(images: serde_json::Value) -> Report {
        serde_json::from_value(json!({
            "backend":"test", "settings":{"region":null,"archive_title":true,
                "threads":8,"warmup":5,"repeat":1}, "initialization_ms":1.0,"images":images,
        }))
        .unwrap()
    }

    fn image(name: &str, recognition: serde_json::Value) -> serde_json::Value {
        json!({"image":name,"size":[1280,720],"recognition":recognition,"stable_text":true,
            "first_ms":1.0,"samples_ms":[1.0],"mean_ms":1.0,"p50_ms":1.0,"p95_ms":1.0})
    }

    #[test]
    fn comparison_pairs_by_name_and_distinguishes_missing_from_empty_text() {
        let empty = json!({"text":"","score":0.0});
        let title = json!({"text":"档案标题","score":0.9});
        let a = report(json!([
            image("a.png", serde_json::Value::Null),
            image("b.png", title.clone())
        ]));
        let b = report(json!([image("b.png", title), image("a.png", empty)]));
        let compared = compare_reports(a, b).unwrap();
        assert_eq!(compared.json["changed_texts"], 1);
        assert_eq!(compared.json["images"][0]["image"], "a.png");
        assert_eq!(compared.json["images"][1]["same_text"], true);
    }

    #[test]
    fn comparison_rejects_incomplete_input_sets() {
        let a = report(json!([image("a.png", serde_json::Value::Null)]));
        let b = report(json!([image("b.png", serde_json::Value::Null)]));
        assert!(compare_reports(a, b).is_err());
    }
}
