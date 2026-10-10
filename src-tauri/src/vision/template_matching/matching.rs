use anyhow::Result;
use image::{GenericImageView, Pixel};

use super::template_source::TemplateProvider;
use crate::utils::region::Region2D;
use crate::vision::ImageRegion;

/// 模板匹配结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MatchResult {
    /// 匹配区域，相对于搜索区域借用的原图坐标。
    pub region: Region2D<u32>,
    /// 匹配得分（-1 ~ 1，越高越像）
    pub score: f32,
}

/// 通过模板名称取得模板，并在已校验的区域内搜索。结果坐标相对于原图。
pub(crate) fn find<I, P>(
    search: &ImageRegion<'_, I>,
    template_name: &str,
    templates: &mut P,
) -> Result<MatchResult>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    P: TemplateProvider + ?Sized,
{
    let template = templates.get(template_name)?;
    super::pure::match_template(search, template)
}

/// 使用已加载模板的纯计算接口。
pub(crate) mod pure {
    use anyhow::{Result, bail};
    use image::{GenericImageView, Pixel, imageops};
    use imageproc::template_matching;

    use super::{super::ccoeff, ImageRegion, MatchResult, Region2D};

    /// 使用已加载的模板在已校验的区域内搜索。
    ///
    /// 结果区域始终相对于原图，而非区域视图。
    pub(crate) fn match_template<I, T>(
        search: &ImageRegion<'_, I>,
        template: &T,
    ) -> Result<MatchResult>
    where
        I: GenericImageView,
        I::Pixel: Pixel<Subpixel = u8>,
        T: GenericImageView,
        T::Pixel: Pixel<Subpixel = u8>,
    {
        let image_region = search.view();
        let matched = match_in_region(&*image_region, template)?;
        Ok(offset_match(matched, search.bounds()))
    }

    fn offset_match(mut matched: MatchResult, search_region: Region2D<u32>) -> MatchResult {
        matched.region = Region2D::from_ltrb(
            search_region.x0() + matched.region.x0(),
            search_region.y0() + matched.region.y0(),
            search_region.x0() + matched.region.x1(),
            search_region.y0() + matched.region.y1(),
        );
        matched
    }

    /// 使用已加载的模板在已经裁剪的图片区域内搜索。
    ///
    /// 使用 `CCOEFF_NORMED`（Pearson 相关系数）。结果区域相对于 `image_region`。
    pub(super) fn match_in_region<I, T>(image_region: &I, template: &T) -> Result<MatchResult>
    where
        I: GenericImageView,
        I::Pixel: Pixel<Subpixel = u8>,
        T: GenericImageView,
        T::Pixel: Pixel<Subpixel = u8>,
    {
        let search_gray = imageops::grayscale(image_region);
        let template_gray = imageops::grayscale(template);

        if template_gray.width() > search_gray.width()
            || template_gray.height() > search_gray.height()
        {
            bail!(
                "template size ({}, {}) is larger than search region size ({}, {})",
                template_gray.width(),
                template_gray.height(),
                search_gray.width(),
                search_gray.height()
            );
        }

        let result = ccoeff::match_template_ccoeff_normed_parallel(&search_gray, &template_gray);
        let extremes = template_matching::find_extremes(&result);

        let (rx, ry) = extremes.max_value_location;
        let region = Region2D::from_ltwh(rx, ry, template.width(), template.height());
        let score = extremes.max_value;

        Ok(MatchResult { region, score })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn maps_search_view_coordinates_back_to_the_original_image() {
        let template = RgbImage::from_fn(2, 2, |x, y| Rgb([30 + (x + y * 2) as u8 * 50; 3]));
        let mut image = RgbImage::new(6, 5);
        image::imageops::replace(&mut image, &template, 3, 2);
        let region = ImageRegion::new(&image, Region2D::from_ltwh(2, 1, 4, 4)).unwrap();
        let matched = pure::match_template(&region, &template).unwrap();
        assert_eq!(matched.region, Region2D::from_ltwh(3, 2, 2, 2));
        assert!((matched.score - 1.0).abs() < 0.001);
    }

    #[test]
    fn rejects_a_template_larger_than_the_valid_search_region() {
        let image = RgbImage::new(2, 2);
        let region = ImageRegion::new(&image, Region2D::from_ltwh(0, 0, 1, 1)).unwrap();
        assert!(pure::match_template(&region, &image).is_err());
    }
}
