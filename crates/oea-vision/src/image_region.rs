//! 识别算法共享的已校验图片区域。

use anyhow::ensure;
use image::{GenericImageView, SubImage, imageops};

use crate::PixelRect;

/// 借用图片中的非空有效区域。构造时检查边界，字段私有以保持校验结果有效。
pub struct ImageRegion<'a, I> {
    image: &'a I,
    bounds: PixelRect,
}

impl<'a, I: GenericImageView> ImageRegion<'a, I> {
    /// 区域使用半开区间，必须非空且完整位于图片内。非法区域返回错误，不自动裁剪。
    pub fn new(image: &'a I, bounds: PixelRect) -> anyhow::Result<Self> {
        ensure!(
            bounds.width > 0 && bounds.height > 0,
            "图像区域必须非空且边界有序: {bounds:?}"
        );
        ensure!(
            bounds.left <= image.width()
                && bounds.width <= image.width() - bounds.left
                && bounds.top <= image.height()
                && bounds.height <= image.height() - bounds.top,
            "图像区域 {bounds:?} 超出图片 {}×{}",
            image.width(),
            image.height()
        );
        Ok(Self { image, bounds })
    }

    /// 区域在原图中的坐标，供识别结果映射回原图。
    pub(crate) fn bounds(&self) -> PixelRect {
        self.bounds
    }

    /// 返回零拷贝的区域视图，其局部坐标从 `(0, 0)` 开始。
    pub(crate) fn view(&self) -> SubImage<&'a I> {
        imageops::crop_imm(
            self.image,
            self.bounds.left,
            self.bounds.top,
            self.bounds.width,
            self.bounds.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn borrows_the_requested_view_with_local_coordinates() {
        let image = RgbaImage::from_fn(4, 3, |x, y| Rgba([x as u8, y as u8, 0, 255]));
        let bounds = PixelRect::new(1, 1, 3, 2);
        let region = ImageRegion::new(&image, bounds).unwrap();
        assert_eq!(region.bounds(), bounds);
        let view = region.view();
        assert_eq!(view.dimensions(), (3, 2));
        assert_eq!(view.get_pixel(0, 0), Rgba([1, 1, 0, 255]));
        assert_eq!(view.get_pixel(2, 1), Rgba([3, 2, 0, 255]));
    }

    #[test]
    fn rejects_invalid_regions_at_construction() {
        let image = RgbaImage::new(3, 3);
        for region in [
            PixelRect::new(1, 1, 0, 1),
            PixelRect::new(1, 1, 1, 0),
            PixelRect::new(2, 2, 2, 1),
            PixelRect::new(2, 2, 1, 2),
            PixelRect::new(u32::MAX, 0, 2, 1),
            PixelRect::new(1, 0, u32::MAX, 1),
        ] {
            assert!(ImageRegion::new(&image, region).is_err());
        }
    }
}
