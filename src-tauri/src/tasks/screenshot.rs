use std::io::Cursor;

use anyhow::{Context, Result};
use base64::Engine;
use image::{ImageFormat, imageops};
use serde::Deserialize;

use crate::{screencap::PrintWindowScreencap, window};

/// 截图编码格式（与前端 `ScreenshotFormat` 对应，值为小写字符串）。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

impl ScreenshotFormat {
    /// 转换为 `image` crate 的编码格式。
    fn to_image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Webp => ImageFormat::WebP,
        }
    }
}

/// 截图的实际实现（返回 anyhow 错误，由命令层转换为 `String` 供 Tauri IPC 使用）。
pub fn capture_screenshot(width: u32, height: u32, format: ScreenshotFormat) -> Result<String> {
    // 1. 定位游戏窗口（PrintWindow 可捕获非最小化后台窗口）
    let hwnd = window::get_window_by_title(
        Some(window::ENDFIELD_WINDOW_CLASS),
        Some(window::ENDFIELD_WINDOW_TITLE),
    )
    .context("未找到游戏窗口")?;

    // 2. 截图
    let mut screencap = PrintWindowScreencap::new(hwnd);
    let raw = screencap.screencap().context("截图失败")?;

    // 3. 缩放到指定尺寸
    let resized = imageops::resize(
        &raw,
        width.max(1),
        height.max(1),
        imageops::FilterType::Triangle,
    );

    // 4. 按格式编码（JPEG 不支持 alpha 通道，先转 RGB 再编码）
    let image_format = format.to_image_format();
    let mut buf = Cursor::new(Vec::new());

    match format {
        ScreenshotFormat::Jpeg => {
            image::DynamicImage::ImageRgba8(resized)
                .to_rgb8()
                .write_to(&mut buf, image_format)
                .context("图片编码失败")?;
        }
        ScreenshotFormat::Png | ScreenshotFormat::Webp => {
            resized
                .write_to(&mut buf, image_format)
                .context("图片编码失败")?;
        }
    }

    // 5. base64 编码返回
    Ok(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}
