use anyhow::{Result, bail};
use image::{GenericImageView, Pixel, imageops};
use imageproc::template_matching;

use super::ccoeff;
use crate::utils::region::Region2D;

/// 模板匹配结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchResult {
    /// 匹配区域（相对于 `image` 的坐标）
    pub region: Region2D<u32>,
    /// 匹配得分（-1 ~ 1，越高越像）
    pub score: f32,
}

/// 在 image 的 search_region 区域内搜索模板。
///
/// 使用 CCOEFF_NORMED（Pearson 相关系数）。
/// `image` / `template` 只需实现 [`GenericImageView`]（`&RgbaImage`、`&RgbImage` 均可），
/// 内部直接对（裁剪视图）灰度化，调用方无需先做颜色转换。
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
    let search_region =
        search_region.unwrap_or_else(|| Region2D::from_ltwh(0, 0, image.width(), image.height()));

    // 直接对裁剪视图灰度化：`SubImage` 解引用到 `SubImageInner`（实现了
    // `GenericImageView`），无需 to_image() 拷贝，也不强制 'static 生命周期。
    let search_gray = imageops::grayscale(&*imageops::crop_imm(
        image,
        search_region.x0(),
        search_region.y0(),
        search_region.width(),
        search_region.height(),
    ));
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
    let region = Region2D::from_ltwh(
        search_region.x0() + rx,
        search_region.y0() + ry,
        template.width(),
        template.height(),
    );
    let score = extremes.max_value;

    Ok(MatchResult { region, score })
}
