use crate::{ImageRegion, PixelRect};

/// 模板匹配结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchResult {
    /// 匹配区域，相对于搜索区域借用的原图坐标。
    pub region: PixelRect,
    /// 匹配得分（-1 ~ 1，越高越像）
    pub score: f32,
}

use anyhow::bail;
use image::{GenericImageView, Pixel, imageops};
use imageproc::template_matching;

use super::ccoeff;

/// 使用已加载的模板在已校验的区域内搜索。
///
/// 结果区域始终相对于原图，而非区域视图。
pub fn match_template<I, T>(
    search: &ImageRegion<'_, I>,
    template: &T,
) -> anyhow::Result<MatchResult>
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

fn offset_match(mut matched: MatchResult, search_region: PixelRect) -> MatchResult {
    matched.region.left += search_region.left;
    matched.region.top += search_region.top;
    matched
}

/// 使用已加载的模板在已经裁剪的图片区域内搜索。
///
/// 使用 `CCOEFF_NORMED`（Pearson 相关系数）。结果区域相对于 `image_region`。
fn match_in_region<I, T>(image_region: &I, template: &T) -> anyhow::Result<MatchResult>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    T: GenericImageView,
    T::Pixel: Pixel<Subpixel = u8>,
{
    let search_gray = imageops::grayscale(image_region);
    let template_gray = imageops::grayscale(template);

    if template_gray.width() > search_gray.width() || template_gray.height() > search_gray.height()
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
    let region = PixelRect::new(rx, ry, template.width(), template.height());
    let score = extremes.max_value;

    Ok(MatchResult { region, score })
}
