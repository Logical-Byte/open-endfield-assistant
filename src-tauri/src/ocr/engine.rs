use std::path::Path;

use anyhow::{Result, anyhow};
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
    pub fn new(pipeline_config: PipelineConfig) -> Result<Self> {
        let model_dir = Path::new("models");
        let model_set_name = DEFAULT_MODEL_SET_NAME;
        let model_set = model_set_by_name(model_set_name)
            .ok_or_else(|| anyhow!("unknown model set {model_set_name:?}"))?;

        let cache = ModelCache::new(model_dir);
        cache.ensure_model_set_for_pipeline(
            model_set,
            pipeline_config,
            ModelDownloadMode::Missing,
        )?;

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
        let ocr = RapidOcr::from_config(cfg)?;

        Ok(Self { ocr })
    }

    pub fn ocr(&mut self, image: &RgbImage) -> Result<OcrOutput> {
        let output = self.ocr.run_image(image)?;
        Ok(output)
    }
}
