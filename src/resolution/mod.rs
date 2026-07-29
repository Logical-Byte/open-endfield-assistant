//! 多分辨率支持模块。
//!
//! 所有识别（模板匹配、OCR、颜色判断）均在 1280×720 基准分辨率上进行，
//! 点击/输入时需要将 720p 坐标缩放到实际分辨率。
//! 仅支持 16:9 分辨率。

use anyhow::{Result, bail};
use image::{RgbaImage, imageops};

/// 游戏窗口分辨率（仅支持 16:9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameResolution {
    /// 实际宽度（像素）
    pub width: u32,
    /// 实际高度（像素）
    pub height: u32,
}

impl GameResolution {
    /// 基准分辨率 1280×720，所有模板图片和坐标均基于此分辨率。
    pub const BASE: Self = GameResolution {
        width: 1280,
        height: 720,
    };

    /// 创建分辨率并验证是否为 16:9。
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let res = Self { width, height };
        res.validate()?;
        Ok(res)
    }

    /// 验证是否为 16:9 比例。如果窗口大小为 0 也认为是合法的（尚未获取到窗口大小时）。
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }
        // 允许 1 像素的误差（因为某些窗口 API 可能返回奇数尺寸）
        let expected_height = self.width * 9 / 16;
        let diff = if self.height > expected_height {
            self.height - expected_height
        } else {
            expected_height - self.height
        };
        if diff > 1 {
            bail!(
                "不支持的分辨率 {}×{}，仅支持 16:9 (预期高度约 {})",
                self.width,
                self.height,
                expected_height
            );
        }
        Ok(())
    }

    /// 将 720p 基准坐标系中的 x 坐标缩放到当前分辨率。
    pub fn scale_x(&self, x: u32) -> u32 {
        (x as u64 * self.width as u64 / 1280) as u32
    }

    /// 将 720p 基准坐标系中的 y 坐标缩放到当前分辨率。
    pub fn scale_y(&self, y: u32) -> u32 {
        (y as u64 * self.height as u64 / 720) as u32
    }

    /// 将 720p 基准坐标系中的点缩放到当前分辨率。
    pub fn scale_point(&self, x: u32, y: u32) -> (u32, u32) {
        (self.scale_x(x), self.scale_y(y))
    }

    /// 将 720p 基准坐标系中的矩形区域缩放到当前分辨率。
    /// 返回 (scaled_left, scaled_top, scaled_right, scaled_bottom)。
    pub fn scale_ltrb(&self, left: u32, top: u32, right: u32, bottom: u32) -> (u32, u32, u32, u32) {
        (
            self.scale_x(left),
            self.scale_y(top),
            self.scale_x(right),
            self.scale_y(bottom),
        )
    }

    /// 将截图缩放到 720p 基准分辨率，用于识别。
    /// 如果已经是基准分辨率则直接返回原图。
    pub fn scale_screenshot_to_base(&self, image: &RgbaImage) -> RgbaImage {
        if self.width == 1280 && self.height == 720 {
            return image.clone();
        }
        imageops::resize(image, 1280, 720, imageops::FilterType::Lanczos3)
    }
}
