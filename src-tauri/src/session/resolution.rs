//! 720p 识别坐标与游戏窗口物理坐标之间的转换。

use anyhow::{Result, bail};
use image::{
    RgbaImage,
    imageops::{self, FilterType},
};

use crate::{automation::Point720p, utils::point::Point2D};

/// 非零的像素尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Resolution {
    width: u32,
    height: u32,
}

impl Resolution {
    /// 创建像素尺寸。
    pub(super) fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            bail!("分辨率尺寸不能为零");
        }

        Ok(Self { width, height })
    }

    pub(super) fn width(&self) -> u32 {
        self.width
    }

    pub(super) fn height(&self) -> u32 {
        self.height
    }
}

/// 模板、识别区域和工作流坐标共同使用的 720p 基准尺寸。
static CANONICAL_RESOLUTION: Resolution = Resolution {
    width: 1280,
    height: 720,
};

/// 固定一次游戏会话使用的物理分辨率，并负责双向坐标空间转换。
pub(super) struct ResolutionTransform {
    physical: Resolution,
}

impl ResolutionTransform {
    /// 创建转换器，并确认物理分辨率与 720p 识别空间的比例兼容。
    pub(super) fn new(physical: Resolution) -> Result<Self> {
        let transform = Self { physical };
        let canonical = transform.canonical();
        let physical = transform.physical();
        let width_scaled = u64::from(physical.width) * u64::from(canonical.height);
        let height_scaled = u64::from(physical.height) * u64::from(canonical.width);

        if width_scaled.abs_diff(height_scaled) > u64::from(canonical.width) {
            bail!(
                "实际窗口分辨率 {}×{} 与 {}×{} 基准比例不兼容，期待 16:9 分辨率",
                physical.width,
                physical.height,
                canonical.width,
                canonical.height,
            );
        }

        Ok(transform)
    }

    /// 将调用方保证有效的 720p 识别坐标翻译为物理窗口坐标。
    pub(super) fn to_physical(&self, point: Point720p) -> Point2D<i32> {
        let canonical = self.canonical();
        let physical = self.physical();

        Point2D {
            x: (u64::from(point.x) * u64::from(physical.width) / u64::from(canonical.width)) as i32,
            y: (u64::from(point.y) * u64::from(physical.height) / u64::from(canonical.height))
                as i32,
        }
    }

    /// 校验物理截图尺寸，并将其归一化到 720p 识别空间。
    pub(super) fn to_canonical_image(&self, image: RgbaImage) -> Result<RgbaImage> {
        let physical = self.physical();
        if image.width() != physical.width || image.height() != physical.height {
            bail!(
                "截图尺寸 {}×{} 与连接时记录的游戏分辨率 {}×{} 不一致",
                image.width(),
                image.height(),
                physical.width,
                physical.height,
            );
        }

        let canonical = self.canonical();
        if physical == canonical {
            return Ok(image);
        }

        Ok(imageops::resize(
            &image,
            canonical.width,
            canonical.height,
            FilterType::Lanczos3,
        ))
    }

    fn canonical(&self) -> &Resolution {
        &CANONICAL_RESOLUTION
    }

    fn physical(&self) -> &Resolution {
        &self.physical
    }
}

#[cfg(test)]
mod tests {
    use image::RgbaImage;

    use crate::{automation::Point720p, utils::point::Point2D};

    use super::{Resolution, ResolutionTransform};

    #[test]
    fn resolution_rejects_zero_dimensions() {
        assert!(Resolution::new(0, 720).is_err());
        assert!(Resolution::new(1280, 0).is_err());
    }

    #[test]
    fn transform_accepts_compatible_resolutions() {
        let full_hd = Resolution::new(1920, 1080).unwrap();
        let rounded_768p = Resolution::new(1366, 768).unwrap();

        assert!(ResolutionTransform::new(full_hd).is_ok());
        assert!(ResolutionTransform::new(rounded_768p).is_ok());
    }

    #[test]
    fn transform_rejects_incompatible_resolution() {
        let incompatible = Resolution::new(1600, 1000).unwrap();

        assert!(ResolutionTransform::new(incompatible).is_err());
    }

    #[test]
    fn point_translation_preserves_floor_rounding() {
        let transform = ResolutionTransform::new(Resolution::new(1920, 1080).unwrap()).unwrap();

        assert_eq!(
            transform.to_physical(Point720p { x: 401, y: 182 }),
            Point2D { x: 601, y: 273 },
        );
    }

    #[test]
    fn image_translation_rejects_changed_physical_size() {
        let transform = ResolutionTransform::new(Resolution::new(1920, 1080).unwrap()).unwrap();
        let resized_window_image = RgbaImage::new(1600, 900);

        assert!(transform.to_canonical_image(resized_window_image).is_err());
    }

    #[test]
    fn image_translation_normalizes_to_canonical_size() {
        let transform = ResolutionTransform::new(Resolution::new(1920, 1080).unwrap()).unwrap();
        let physical_image = RgbaImage::new(1920, 1080);

        let canonical_image = transform.to_canonical_image(physical_image).unwrap();

        assert_eq!(canonical_image.dimensions(), (1280, 720));
    }

    #[test]
    fn canonical_image_keeps_its_pixels() {
        let transform = ResolutionTransform::new(Resolution::new(1280, 720).unwrap()).unwrap();
        let mut image = RgbaImage::new(1280, 720);
        image.put_pixel(401, 182, image::Rgba([12, 34, 56, 78]));

        let translated = transform.to_canonical_image(image).unwrap();

        assert_eq!(
            *translated.get_pixel(401, 182),
            image::Rgba([12, 34, 56, 78])
        );
    }
}
