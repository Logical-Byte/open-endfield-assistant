//! PP-OCR 单行识别预处理与贪心 CTC 解码。
//!
//! 缩放和解码规则参考 rapidocr-core 0.2.2（Apache-2.0）：
//! https://github.com/White-NX/rapidocr-rs/blob/main/crates/rapidocr-core/src/rec.rs

use anyhow::{Result, ensure};
use image::RgbImage;

use super::Recognition;

const HEIGHT: usize = 48;
const MIN_WIDTH: usize = 320;

pub(super) fn preprocess(image: &RgbImage) -> Result<([usize; 4], Vec<f32>)> {
    ensure!(
        image.width() > 0 && image.height() > 0,
        "OCR 输入图片尺寸不能为空"
    );
    let ratio = image.width() as f32 / image.height() as f32;
    let padded_width = (HEIGHT as f32 * ratio.max(MIN_WIDTH as f32 / HEIGHT as f32)) as usize;
    let width = ((HEIGHT as f32 * ratio).ceil() as usize)
        .min(padded_width)
        .max(1);
    let mut pixels = vec![0.0; 3 * HEIGHT * padded_width];

    // 使用 half-pixel 双线性采样，并先舍入到 u8，保持现有 OCR 输入一致。
    // image::resize 的滤波细节会改变 logits，因此这里明确写出采样规则。
    for y in 0..HEIGHT {
        let (y0, y1, fy) = sample_bounds(y, HEIGHT, image.height());
        for x in 0..width {
            let (x0, x1, fx) = sample_bounds(x, width, image.width());
            for channel in 0..3 {
                let source_channel = 2 - channel; // 模型使用 BGR。
                let top = image.get_pixel(x0, y0)[source_channel] as f32 * (1.0 - fx)
                    + image.get_pixel(x1, y0)[source_channel] as f32 * fx;
                let bottom = image.get_pixel(x0, y1)[source_channel] as f32 * (1.0 - fx)
                    + image.get_pixel(x1, y1)[source_channel] as f32 * fx;
                let value = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0);
                pixels[(channel * HEIGHT + y) * padded_width + x] = value / 255.0 / 0.5 - 1.0;
            }
        }
    }
    Ok(([1, 3, HEIGHT, padded_width], pixels))
}

fn sample_bounds(destination: usize, output_len: usize, input_len: u32) -> (u32, u32, f32) {
    let scale = input_len as f32 / output_len as f32;
    let source = (destination as f32 + 0.5) * scale - 0.5;
    if source <= 0.0 || input_len == 1 {
        return (0, 0, 0.0);
    }
    let low = source.floor() as u32;
    if low >= input_len - 1 {
        return (input_len - 1, input_len - 1, 0.0);
    }
    (low, low + 1, source - low as f32)
}

pub(super) fn decode(probabilities: &[f32], characters: &[String]) -> Recognition {
    let mut text = String::new();
    let mut total_score = 0.0;
    let mut count = 0;
    let mut previous = usize::MAX;
    for step in probabilities.chunks_exact(characters.len()) {
        let (index, probability) = step
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .unwrap();
        if index != 0 && index != previous {
            text.push_str(&characters[index]);
            total_score += probability;
            count += 1;
        }
        previous = index;
    }
    if text.trim().is_empty() {
        text.clear();
        count = 0;
    }
    Recognition {
        text,
        score: if count == 0 {
            0.0
        } else {
            total_score / count as f32
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctc_blank_separates_repeated_characters() {
        let characters = ["", "哈", " "].map(str::to_owned);
        // 哈、哈、blank、哈、空格：连续重复折叠，blank 后的重复字符保留。
        let output = decode(
            &[
                0.0, 0.9, 0.1, 0.0, 0.8, 0.2, 0.9, 0.1, 0.0, 0.0, 0.8, 0.2, 0.0, 0.1, 0.9,
            ],
            &characters,
        );
        assert_eq!(output.text, "哈哈 ");
        assert!((output.score - (0.9 + 0.8 + 0.9) / 3.0).abs() < 1e-6);
        let whitespace = decode(&[0.0, 0.1, 0.9], &characters);
        assert_eq!(whitespace.text, "");
        assert_eq!(whitespace.score, 0.0);
    }
}
