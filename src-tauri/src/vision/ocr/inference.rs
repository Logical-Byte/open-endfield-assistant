use std::path::Path;

use anyhow::{Context, Result};
use image::RgbImage;
use rapidocr_core::{
    RapidOcr,
    config::{InferenceOptions, PipelineConfig},
    model::{ModelCache, ModelDownloadMode, PPOCRV6_TINY},
};

use super::{Config, Recognition};

pub(super) const NAME: &str = "rapidocr";

/// 将第三方识别配置和结果限制在 OCR 模块内。
pub(super) struct Inference {
    ocr: RapidOcr,
}

impl Inference {
    pub(super) fn new(models_dir: &Path, config: Config) -> Result<Self> {
        let pipeline = PipelineConfig::recognition_only();
        let cache = ModelCache::new(models_dir);
        cache.ensure_model_set_for_pipeline(&PPOCRV6_TINY, pipeline, ModelDownloadMode::Never)?;
        let config = cache
            .config_for(&PPOCRV6_TINY)
            .with_pipeline(pipeline)
            .with_inference_options(InferenceOptions {
                intra_threads: config.threads,
                inter_threads: 1,
                parallel_execution: true,
                enable_cpu_mem_arena: true,
                ..Default::default()
            });
        let ocr = RapidOcr::from_config(config).context("创建 OCR 推理引擎失败（ONNX Runtime）")?;
        Ok(Self { ocr })
    }

    pub(super) fn recognize(&mut self, image: &RgbImage) -> Result<Recognition> {
        let output = self.ocr.run_image(image)?;
        Ok(Recognition {
            text: output
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            // recognition_only 管线最多返回一行，没有结果时置信度为零。
            score: output.lines.first().map_or(0.0, |line| line.score),
        })
    }
}
