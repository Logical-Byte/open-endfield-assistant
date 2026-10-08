use std::{fs, path::Path, sync::Arc};

use anyhow::{Context, Result, ensure};
use image::RgbImage;
use rten::{Model, ModelOptions, RunOptions, ThreadPool, Value};

use super::{Config, Recognition, recognition};

pub(super) const NAME: &str = "rten";

/// PP-OCRv6 tiny 的 RTen 识别实现。调用方只使用 OCR 模块的图像接口。
pub(super) struct Inference {
    model: Model,
    options: RunOptions,
    characters: Vec<String>,
}

impl Inference {
    pub(super) fn new(models_dir: &Path, config: Config) -> Result<Self> {
        let dict = fs::read_to_string(models_dir.join("ppocrv6_tiny_dict.txt"))?;
        ensure!(!dict.trim().is_empty(), "OCR 字典为空");
        let characters = std::iter::once(String::new())
            .chain(dict.lines().map(|line| line.trim_end().to_owned()))
            .chain(std::iter::once(" ".to_owned()))
            .collect();
        // PP-OCRv6 tiny 使用的算子，避免将无关实现链接进应用。
        let operators = rten::op_registry!(
            Add,
            AveragePool,
            BatchNormalization,
            Conv,
            Div,
            Erf,
            HardSigmoid,
            Identity,
            MatMul,
            Mul,
            ReduceMean,
            Relu,
            Softmax,
            Squeeze,
            Transpose,
            Unsqueeze
        );
        let model = ModelOptions::with_ops(operators)
            .load_file(models_dir.join("PP-OCRv6_rec_tiny.onnx"))
            .context("创建 OCR 推理引擎失败（RTen）")?;
        let options = RunOptions::default()
            .with_thread_pool(Some(Arc::new(ThreadPool::with_num_threads(config.threads))));
        Ok(Self {
            model,
            options,
            characters,
        })
    }

    pub(super) fn recognize(&mut self, image: &RgbImage) -> Result<Recognition> {
        let (shape, pixels) = recognition::preprocess(image)?;
        let input = Value::from_shape(shape, pixels)?;
        let output = self
            .model
            .run_one(input.into(), Some(self.options.clone()))?;
        let (shape, probabilities) = output.into_shape_vec::<f32, 3>()?;
        ensure!(
            shape[0] == 1 && shape[2] == self.characters.len(),
            "OCR 输出形状 {:?} 与字典字符数 {} 不符",
            shape,
            self.characters.len()
        );
        Ok(recognition::decode(&probabilities, &self.characters))
    }
}
