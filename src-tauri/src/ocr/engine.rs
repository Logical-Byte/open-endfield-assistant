use std::{path::Path, time::Instant};

use anyhow::{Context, Result, anyhow};
use image::RgbImage;
use rapidocr_core::{
    RapidOcr,
    config::{InferenceOptions, PipelineConfig},
    model::{DEFAULT_MODEL_SET_NAME, ModelCache, ModelDownloadMode, model_set_by_name},
    types::OcrOutput,
};

pub struct OcrEngine {
    ocr: RapidOcr,
}

impl OcrEngine {
    /// 创建 OCR 引擎。
    ///
    /// # 参数
    /// - `pipeline_config`: 识别管线配置
    /// - `models_dir`: OCR 模型目录（如 [`crate::app_paths::AppPaths::models_dir()`]）
    pub fn new(pipeline_config: PipelineConfig, models_dir: &Path) -> Result<Self> {
        let model_dir = models_dir;
        let model_set_name = DEFAULT_MODEL_SET_NAME;
        let model_set = model_set_by_name(model_set_name)
            .ok_or_else(|| anyhow!("unknown model set {model_set_name:?}"))?;

        let cache = ModelCache::new(model_dir);
        cache
            .ensure_model_set_for_pipeline(model_set, pipeline_config, ModelDownloadMode::Never)
            .with_context(|| {
                format!(
                    "初始化 OCR 模型失败（模型目录: {}），请确认 models 目录包含识别模型文件",
                    model_dir.display()
                )
            })?;

        let cfg = cache
            .config_for(model_set)
            .with_pipeline(pipeline_config)
            .with_inference_options(InferenceOptions {
                intra_threads: 8,
                inter_threads: 1,
                parallel_execution: true,
                enable_cpu_mem_arena: true,
                ..Default::default()
            });
        let ocr =
            RapidOcr::from_config(cfg).with_context(|| "创建 OCR 推理引擎失败（ONNX Runtime）")?;

        Ok(Self { ocr })
    }

    pub fn ocr(&mut self, image: &RgbImage) -> Result<OcrOutput> {
        let start_time = Instant::now();
        let output = self.ocr.run_image(image)?;
        let elapsed = start_time.elapsed();
        tracing::trace!(
            "OCR completed in {:.2?}, output: {:?}",
            elapsed,
            output
                .lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidocr_core::config::PipelineConfig;

    /// 模型缺失（用户最常遇到的 setup 失败场景）时，错误链必须带可读的上下文：
    /// 说明是"初始化 OCR 模型"失败，并给出模型目录路径，便于用户 / 开发者排查。
    #[test]
    fn model_missing_error_has_context() {
        let missing_dir = std::env::temp_dir().join("oea-test-missing-models");
        let err = match OcrEngine::new(PipelineConfig::recognition_only(), &missing_dir) {
            Ok(_) => panic!("模型缺失时应返回错误"),
            Err(e) => e,
        };
        let chain = format!("{err:#}");
        assert!(
            chain.contains("初始化 OCR 模型失败"),
            "错误链应包含 OCR 初始化上下文，实际: {chain}"
        );
        assert!(
            chain.contains("missing-models"),
            "错误链应包含模型目录路径，实际: {chain}"
        );
    }
}
