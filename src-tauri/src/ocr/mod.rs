//! OCR 模块（基础设施，通用库）。
//!
//! - [`OcrEngine`]：基于 `rapidocr`（PP-OCRv6 模型）的识别引擎，由 `Session` 共享复用；
//! - [`text_detection`]：单行文本检测（阈值二值化），供 OCR 前裁剪行区域。

pub mod engine;
pub mod text_detection;

pub use engine::OcrEngine;
