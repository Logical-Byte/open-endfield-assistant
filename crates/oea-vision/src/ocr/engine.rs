use std::{path::Path, time::Instant};

use anyhow::Context;
use image::{RgbImage, RgbaImage, imageops};
use imageproc::contrast::ThresholdType;

use crate::ImageRegion;

use super::{Config, Recognition, inference, text_detection};

pub struct OcrEngine {
    inference: inference::Inference,
}

impl OcrEngine {
    /// 加载 PP-OCRv6 tiny 模型和字典，初始化可复用的识别引擎。
    ///
    /// 目录须包含 `PP-OCRv6_rec_tiny.onnx` 和 `ppocrv6_tiny_dict.txt`。
    /// 文件读取或模型初始化失败时返回带上下文的错误。每个实例持有自己的推理线程池。
    pub fn new(models_dir: &Path, config: Config) -> anyhow::Result<Self> {
        let inference = inference::Inference::new(models_dir, config).with_context(|| {
            format!(
                "初始化 OCR 模型失败（模型目录: {}），请确认识别模型和字典完整",
                models_dir.display(),
            )
        })?;
        Ok(Self { inference })
    }

    /// 输入一张已裁剪的 RGB 单行图像，返回文字与平均字符置信度。
    pub fn recognize(&mut self, image: &RgbImage) -> anyhow::Result<Recognition> {
        let start = Instant::now();
        let result = self.inference.recognize(image)?;
        tracing::trace!(backend = super::BACKEND_NAME, elapsed = ?start.elapsed(), text = %result.text, score = result.score, "OCR completed");
        Ok(result)
    }

    /// 输入包含单行白色文字的 RGBA 区域，以亮度阈值 `128` 二值化，
    /// 取所有亮像素的包围框并向外扩展 `6` 像素（裁剪到区域范围）后识别。
    /// 区域内没有文字像素时返回 `None`，有像素但未识别出文字时返回空结果。
    pub fn recognize_region(
        &mut self,
        region: &ImageRegion<'_, RgbaImage>,
    ) -> anyhow::Result<Option<Recognition>> {
        let cropped = region.view().to_image();
        let rgb = image::DynamicImage::ImageRgba8(cropped).to_rgb8();
        let Some(text_region) =
            text_detection::detect_single_line(&rgb, 128, ThresholdType::Binary, 6)
        else {
            return Ok(None);
        };
        let cropped = imageops::crop_imm(
            &rgb,
            text_region.left,
            text_region.top,
            text_region.width,
            text_region.height,
        )
        .to_image();
        self.recognize(&cropped).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_missing_error_has_context() {
        let dir = tempfile::tempdir().unwrap();
        let err = OcrEngine::new(&dir.path().join("missing-models"), Config::default())
            .err()
            .unwrap();
        let chain = format!("{err:#}");
        assert!(chain.contains("初始化 OCR 模型失败"), "{chain}");
        assert!(chain.contains("missing-models"), "{chain}");
    }
}
