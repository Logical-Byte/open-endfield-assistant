use std::path::Path;

use anyhow::Result;
use image::{RgbImage, imageops};
use imageproc::template_matching;

use crate::utils::point::Region2D;

pub mod ccoeff;

pub use ccoeff::*;

/// 加载模板图片
pub fn load_template<P>(path: P) -> Result<RgbImage>
where
    P: AsRef<Path>,
{
    let img = image::open(path)?;
    Ok(img.to_rgb8())
}

/// 模板匹配结果
pub struct MatchResult {
    /// 匹配区域（相对于 `image` 的坐标）
    pub region: Region2D<u32>,
    /// 匹配得分（-1 ~ 1，越高越像）
    pub score: f32,
}

/// 在 image 的 search_region 区域内搜索模板。
///
/// 使用 CCOEFF_NORMED（Pearson 相关系数）
pub fn match_template_in_region(
    image: &RgbImage,
    template: &RgbImage,
    search_region: Option<Region2D<u32>>,
) -> Option<MatchResult> {
    let search_region =
        search_region.unwrap_or_else(|| Region2D::from_ltwh(0, 0, image.width(), image.height()));

    let search_img = &imageops::crop_imm(
        image,
        search_region.x0(),
        search_region.y0(),
        search_region.width(),
        search_region.height(),
    )
    .to_image();
    let search_gray = imageops::grayscale(search_img);
    let template_gray = imageops::grayscale(template);

    if template_gray.width() > search_gray.width() || template_gray.height() > search_gray.height()
    {
        return None;
    }

    let result = match_template_ccoeff_normed_parallel(&search_gray, &template_gray);
    let extremes = template_matching::find_extremes(&result);

    let (rx, ry) = extremes.max_value_location;
    let region = Region2D::from_ltwh(
        search_region.x0() + rx,
        search_region.y0() + ry,
        template.width(),
        template.height(),
    );
    let score = extremes.max_value;

    Some(MatchResult { region, score })
}
