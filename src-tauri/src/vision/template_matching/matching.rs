use anyhow::Result;
use image::{GenericImageView, Pixel};

use super::TemplateProvider;
use crate::utils::region::Region2D;

/// 模板匹配结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchResult {
    /// 匹配区域（相对于 `image` 的坐标）
    pub region: Region2D<u32>,
    /// 匹配得分（-1 ~ 1，越高越像）
    pub score: f32,
}

/// 通过模板名称取得模板，并在 `image` 的指定区域内搜索。
pub fn find<I, P>(
    image: &I,
    template_name: &str,
    search_region: Region2D<u32>,
    templates: &mut P,
) -> Result<MatchResult>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    P: TemplateProvider + ?Sized,
{
    let template = templates.get(template_name)?;
    pure::match_template_in_region(image, template, Some(search_region))
}

/// 通过模板名称取得模板，并在已经裁剪的图片区域内搜索。
pub fn find_in_region<I, P>(
    image_region: &I,
    template_name: &str,
    templates: &mut P,
) -> Result<MatchResult>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    P: TemplateProvider + ?Sized,
{
    let template = templates.get(template_name)?;
    pure::match_in_region(image_region, template)
}

/// 使用已加载模板的纯计算接口。
pub mod pure {
    use anyhow::{Result, bail};
    use image::{GenericImageView, Pixel, imageops};
    use imageproc::template_matching;

    use super::{super::ccoeff, MatchResult, Region2D};

    /// 使用已加载的模板在 `image` 的指定区域内搜索。
    ///
    /// `search_region` 为空时搜索完整图片。结果区域始终相对于 `image`。
    pub fn match_template_in_region<I, T>(
        image: &I,
        template: &T,
        search_region: Option<Region2D<u32>>,
    ) -> Result<MatchResult>
    where
        I: GenericImageView,
        I::Pixel: Pixel<Subpixel = u8>,
        T: GenericImageView,
        T::Pixel: Pixel<Subpixel = u8>,
    {
        let Some(search_region) = search_region else {
            return match_in_region(image, template);
        };

        let image_region = imageops::crop_imm(
            image,
            search_region.x0(),
            search_region.y0(),
            search_region.width(),
            search_region.height(),
        );
        let matched = match_in_region(&*image_region, template)?;
        Ok(offset_match(matched, search_region))
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
    /// 使用 CCOEFF_NORMED（Pearson 相关系数）。结果区域相对于 `image_region`。
    pub fn match_in_region<I, T>(image_region: &I, template: &T) -> Result<MatchResult>
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
