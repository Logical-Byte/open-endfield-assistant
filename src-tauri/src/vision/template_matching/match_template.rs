use anyhow::{Result, bail};
use image::{GenericImageView, Pixel, imageops};
use imageproc::template_matching;

use super::{TemplateProvider, ccoeff};
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
pub(crate) fn find<I, P>(
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
    let image_region = imageops::crop_imm(
        image,
        search_region.x0(),
        search_region.y0(),
        search_region.width(),
        search_region.height(),
    );
    let mut matched = find_in_region(&*image_region, template_name, templates)?;
    matched.region = Region2D::from_ltrb(
        search_region.x0() + matched.region.x0(),
        search_region.y0() + matched.region.y0(),
        search_region.x0() + matched.region.x1(),
        search_region.y0() + matched.region.y1(),
    );
    Ok(matched)
}

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
    let mut matched = match_in_region(&*image_region, template)?;
    matched.region = Region2D::from_ltrb(
        search_region.x0() + matched.region.x0(),
        search_region.y0() + matched.region.y0(),
        search_region.x0() + matched.region.x1(),
        search_region.y0() + matched.region.y1(),
    );
    Ok(matched)
}

/// 通过模板名称取得模板，并在已经裁剪的图片区域内搜索。
pub(crate) fn find_in_region<I, P>(
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
    match_in_region(image_region, template)
}

/// 使用已加载的模板在已经裁剪的图片区域内搜索。
///
/// 使用 CCOEFF_NORMED（Pearson 相关系数）。
/// `image_region` / `template` 只需实现 [`GenericImageView`]（`&RgbaImage`、`&RgbImage`
/// 均可），内部直接灰度化，调用方无需先做颜色转换。结果区域相对于
/// `image_region`。
pub fn match_in_region<I, T>(image_region: &I, template: &T) -> Result<MatchResult>
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
    let region = Region2D::from_ltwh(rx, ry, template.width(), template.height());
    let score = extremes.max_value;

    Ok(MatchResult { region, score })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use image::{Rgb, RgbImage};

    use crate::utils::region::Region2D;

    use super::{super::TemplateProvider, find, match_template_in_region};

    struct InMemoryTemplates {
        template: RgbImage,
    }

    impl TemplateProvider for InMemoryTemplates {
        fn get(&mut self, template_name: &str) -> Result<&RgbImage> {
            assert_eq!(template_name, "foo/bar.png");
            Ok(&self.template)
        }
    }

    #[test]
    fn finds_named_template_in_search_region() {
        let image = RgbImage::from_fn(4, 2, |x, y| {
            let value = match (x, y) {
                (1, 0) | (2, 1) => 0,
                (2, 0) | (1, 1) => 255,
                _ => 128,
            };
            Rgb([value, value, value])
        });
        let mut templates = InMemoryTemplates {
            template: RgbImage::from_fn(2, 2, |x, y| {
                let value = if x == y { 0 } else { 255 };
                Rgb([value, value, value])
            }),
        };

        let matched = find(
            &image,
            "foo/bar.png",
            Region2D::from_ltwh(1, 0, 3, 2),
            &mut templates,
        )
        .unwrap();

        assert_eq!(matched.region, Region2D::from_ltwh(1, 0, 2, 2));
        assert_eq!(matched.score, 1.0);
    }

    #[test]
    fn matches_loaded_template_in_search_region() {
        let image = RgbImage::from_fn(4, 2, |x, y| {
            let value = match (x, y) {
                (1, 0) | (2, 1) => 0,
                (2, 0) | (1, 1) => 255,
                _ => 128,
            };
            Rgb([value, value, value])
        });
        let template = RgbImage::from_fn(2, 2, |x, y| {
            let value = if x == y { 0 } else { 255 };
            Rgb([value, value, value])
        });

        let matched =
            match_template_in_region(&image, &template, Some(Region2D::from_ltwh(1, 0, 3, 2)))
                .unwrap();

        assert_eq!(matched.region, Region2D::from_ltwh(1, 0, 2, 2));
        assert_eq!(matched.score, 1.0);
    }
}
