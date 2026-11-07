//! 720p 识别坐标与游戏窗口内容区域物理坐标之间的转换。

use image::{
    RgbaImage,
    imageops::{self, FilterType},
};

use super::Error as GameEnvironmentError;
use crate::automation::Point720p;
use oea_window::{ContentRect, Point};

/// 非零的像素尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Resolution {
    /// 创建像素尺寸。
    pub fn new(width: u32, height: u32) -> Result<Self, GameEnvironmentError> {
        if width == 0 || height == 0 {
            return Err(GameEnvironmentError::UnsupportedResolution { width, height });
        }

        Ok(Self { width, height })
    }

    /// 像素尺寸的宽度与高度。
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

/// 模板、识别区域和工作流坐标共同使用的 720p 基准尺寸。
static CANONICAL_RESOLUTION: Resolution = Resolution {
    width: 1280,
    height: 720,
};

/// 固定一次游戏会话使用的内容区域，并负责双向坐标空间转换。
///
/// 内容区域是游戏画面在窗口客户区中的实际位置。原生客户端的内容区域等于整个客户区，
/// 浏览器中的云游戏只占客户区内的网页视口，上方还有浏览器工具栏。
pub struct ResolutionTransform {
    /// 截图使用的客户区尺寸，用于校验截图尺寸与会话开始时一致。
    physical: Resolution,
    /// 内容区域在客户区中的位置与尺寸。
    content: ContentRect,
}

impl ResolutionTransform {
    /// 创建转换器，并确认内容区域可用且与 720p 识别空间的比例兼容。
    pub fn new(physical: Resolution, content: ContentRect) -> Result<Self, GameEnvironmentError> {
        let transform = Self { physical, content };
        let canonical = transform.canonical();
        let content_size = transform.content_size();
        let right = i64::from(content.x) + i64::from(content.width);
        let bottom = i64::from(content.y) + i64::from(content.height);
        let fits = content.x >= 0
            && content.y >= 0
            && right <= i64::from(physical.width)
            && bottom <= i64::from(physical.height);

        // 内容区域无效时按不支持的分辨率上报：此时截图裁剪与点击映射都不可信。
        if !fits {
            return Err(GameEnvironmentError::UnsupportedResolution {
                width: content.width,
                height: content.height,
            });
        }

        let width_scaled = u64::from(content_size.width) * u64::from(canonical.height);
        let height_scaled = u64::from(content_size.height) * u64::from(canonical.width);

        if width_scaled.abs_diff(height_scaled) > u64::from(canonical.width) {
            return Err(GameEnvironmentError::UnsupportedResolution {
                width: content_size.width,
                height: content_size.height,
            });
        }

        Ok(transform)
    }

    /// 将调用方保证有效的 720p 识别坐标翻译为客户区内的物理坐标。
    pub fn to_physical(&self, point: Point720p) -> Point {
        let canonical = self.canonical();
        let content = self.content;

        Point {
            x: content.x
                + (u64::from(point.x) * u64::from(content.width) / u64::from(canonical.width))
                    as i32,
            y: content.y
                + (u64::from(point.y) * u64::from(content.height) / u64::from(canonical.height))
                    as i32,
        }
    }

    /// 校验物理截图尺寸，裁剪出内容区域，并将其归一化到 720p 识别空间。
    pub fn to_canonical_image(&self, image: RgbaImage) -> Result<RgbaImage, GameEnvironmentError> {
        let physical = self.physical;
        if image.width() != physical.width || image.height() != physical.height {
            return Err(GameEnvironmentError::WindowSizeChanged {
                expected_width: physical.width,
                expected_height: physical.height,
                actual_width: image.width(),
                actual_height: image.height(),
            });
        }

        let content = self.content;
        let cropped = if content.x == 0
            && content.y == 0
            && content.width == physical.width
            && content.height == physical.height
        {
            image
        } else {
            imageops::crop_imm(
                &image,
                content.x as u32,
                content.y as u32,
                content.width,
                content.height,
            )
            .to_image()
        };

        let canonical = self.canonical();
        if cropped.width() == canonical.width && cropped.height() == canonical.height {
            return Ok(cropped);
        }

        Ok(imageops::resize(
            &cropped,
            canonical.width,
            canonical.height,
            FilterType::Lanczos3,
        ))
    }

    fn content_size(&self) -> Resolution {
        // 内容区域尺寸已在 `new` 中确认非零。
        Resolution {
            width: self.content.width,
            height: self.content.height,
        }
    }

    fn canonical(&self) -> &Resolution {
        &CANONICAL_RESOLUTION
    }

    pub fn physical(&self) -> &Resolution {
        &self.physical
    }
}

#[cfg(test)]
mod tests {
    use image::RgbaImage;

    use crate::automation::Point720p;
    use oea_window::{ContentRect, Point};

    use super::{Resolution, ResolutionTransform};

    /// 内容区域占满客户区，对应原生游戏客户端。
    fn full_client(width: u32, height: u32) -> ContentRect {
        ContentRect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn resolution_rejects_zero_dimensions() {
        assert!(Resolution::new(0, 720).is_err());
        assert!(Resolution::new(1280, 0).is_err());
    }

    #[test]
    fn transform_accepts_compatible_resolutions() {
        let full_hd = Resolution::new(1920, 1080).unwrap();
        let rounded_768p = Resolution::new(1366, 768).unwrap();

        assert!(ResolutionTransform::new(full_hd, full_client(1920, 1080)).is_ok());
        assert!(ResolutionTransform::new(rounded_768p, full_client(1366, 768)).is_ok());
    }

    #[test]
    fn transform_accepts_viewport_inside_larger_client() {
        // 云游戏：客户区比网页视口多出浏览器工具栏高度。
        let client = Resolution::new(1280, 777).unwrap();
        let viewport = ContentRect {
            x: 0,
            y: 57,
            width: 1280,
            height: 720,
        };

        assert!(ResolutionTransform::new(client, viewport).is_ok());
    }

    #[test]
    fn transform_rejects_incompatible_resolution() {
        let incompatible = Resolution::new(1600, 1000).unwrap();

        assert!(ResolutionTransform::new(incompatible, full_client(1600, 1000)).is_err());
    }

    #[test]
    fn transform_rejects_content_outside_client() {
        let client = Resolution::new(1280, 777).unwrap();
        // 内容区域下沿超出客户区时裁剪与点击都会越界。
        let beyond_bottom = ContentRect {
            x: 0,
            y: 58,
            width: 1280,
            height: 720,
        };

        assert!(ResolutionTransform::new(client, beyond_bottom).is_err());
    }

    #[test]
    fn point_translation_preserves_floor_rounding() {
        let transform = ResolutionTransform::new(
            Resolution::new(1920, 1080).unwrap(),
            full_client(1920, 1080),
        )
        .unwrap();

        assert_eq!(
            transform.to_physical(Point720p { x: 401, y: 182 }),
            Point { x: 601, y: 273 },
        );
    }

    #[test]
    fn point_translation_respects_content_offset() {
        let client = Resolution::new(1280, 777).unwrap();
        let viewport = ContentRect {
            x: 0,
            y: 57,
            width: 1280,
            height: 720,
        };
        let transform = ResolutionTransform::new(client, viewport).unwrap();

        assert_eq!(
            transform.to_physical(Point720p { x: 176, y: 25 }),
            Point { x: 176, y: 82 },
        );
    }

    #[test]
    fn image_translation_rejects_changed_physical_size() {
        let transform = ResolutionTransform::new(
            Resolution::new(1920, 1080).unwrap(),
            full_client(1920, 1080),
        )
        .unwrap();
        let resized_window_image = RgbaImage::new(1600, 900);

        assert!(transform.to_canonical_image(resized_window_image).is_err());
    }

    #[test]
    fn image_translation_normalizes_to_canonical_size() {
        let transform = ResolutionTransform::new(
            Resolution::new(1920, 1080).unwrap(),
            full_client(1920, 1080),
        )
        .unwrap();
        let physical_image = RgbaImage::new(1920, 1080);

        let canonical_image = transform.to_canonical_image(physical_image).unwrap();

        assert_eq!(canonical_image.dimensions(), (1280, 720));
    }

    #[test]
    fn image_translation_crops_content_offset() {
        let client = Resolution::new(1280, 777).unwrap();
        let viewport = ContentRect {
            x: 0,
            y: 57,
            width: 1280,
            height: 720,
        };
        let transform = ResolutionTransform::new(client, viewport).unwrap();
        let mut image = RgbaImage::new(1280, 777);
        image.put_pixel(401, 57 + 182, image::Rgba([12, 34, 56, 78]));

        let translated = transform.to_canonical_image(image).unwrap();

        assert_eq!(translated.dimensions(), (1280, 720));
        assert_eq!(
            *translated.get_pixel(401, 182),
            image::Rgba([12, 34, 56, 78])
        );
    }

    #[test]
    fn canonical_image_keeps_its_pixels() {
        let transform =
            ResolutionTransform::new(Resolution::new(1280, 720).unwrap(), full_client(1280, 720))
                .unwrap();
        let mut image = RgbaImage::new(1280, 720);
        image.put_pixel(401, 182, image::Rgba([12, 34, 56, 78]));

        let translated = transform.to_canonical_image(image).unwrap();

        assert_eq!(
            *translated.get_pixel(401, 182),
            image::Rgba([12, 34, 56, 78])
        );
    }
}
