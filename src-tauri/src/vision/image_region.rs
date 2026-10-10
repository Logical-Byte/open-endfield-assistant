//! 识别算法共享的已校验图片区域。

use anyhow::ensure;
use image::{GenericImageView, SubImage, imageops};

use crate::utils::region::Region2D;

/// 借用图片中的非空有效区域。构造时检查边界，字段私有以保持校验结果有效。
pub(crate) struct ImageRegion<'a, I> {
    image: &'a I,
    bounds: Region2D<u32>,
}

impl<'a, I: GenericImageView> ImageRegion<'a, I> {
    /// 区域使用半开区间，必须非空且完整位于图片内。非法区域返回错误，不自动裁剪。
    pub(crate) fn new(image: &'a I, bounds: Region2D<u32>) -> anyhow::Result<Self> {
        ensure!(
            bounds.x0() < bounds.x1() && bounds.y0() < bounds.y1(),
            "图像区域必须非空且边界有序: {bounds:?}"
        );
        ensure!(
            bounds.x1() <= image.width() && bounds.y1() <= image.height(),
            "图像区域 {bounds:?} 超出图片 {}×{}",
            image.width(),
            image.height()
        );
        Ok(Self { image, bounds })
    }

    /// 区域在原图中的坐标，供识别结果映射回原图。
    pub(crate) fn bounds(&self) -> Region2D<u32> {
        self.bounds
    }

    /// 返回零拷贝的区域视图，其局部坐标从 `(0, 0)` 开始。
    pub(crate) fn view(&self) -> SubImage<&'a I> {
        imageops::crop_imm(
            self.image,
            self.bounds.x0(),
            self.bounds.y0(),
            self.bounds.width(),
            self.bounds.height(),
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
        let bounds = Region2D::from_ltwh(1, 1, 3, 2);
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
            Region2D::from_ltrb(1, 1, 1, 2),
            Region2D::from_ltrb(2, 1, 1, 2),
            Region2D::from_ltrb(1, 2, 2, 1),
            Region2D::from_ltrb(2, 2, 4, 3),
            Region2D::from_ltrb(2, 2, 3, 4),
        ] {
            assert!(ImageRegion::new(&image, region).is_err());
        }
    }
}
