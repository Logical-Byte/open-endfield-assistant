//! OCR 模块（基础设施，通用库）。
//!
//! `OcrEngine` 提供单行图像识别和截图区域识别。
//! 调用方使用本模块的配置和识别结果，不依赖推理库的类型。

mod engine;
mod inference;
mod recognition;
mod text_detection;

pub(crate) use engine::OcrEngine;

/// 推理后端的诊断名称，用于日志和开发者工具的结果标识。
pub(crate) const BACKEND_NAME: &str = inference::NAME;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Config {
    pub threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { threads: 8 }
    }
}

/// 识别结果。未识别到文字时返回空字符串和零置信度。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct Recognition {
    pub text: String,
    pub score: f32,
}
