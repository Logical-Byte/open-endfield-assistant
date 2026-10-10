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
) -> anyhow::Result<MatchResult>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    P: TemplateProvider + ?Sized,
{
    let template = templates.get(template_name)?;
    super::pure::match_template(search, template)
}

/// 在同一区域搜索多个命名模板，返回最佳模板名称及其匹配位置、分数。
///
/// 候选为空时返回 `Ok(None)`。分数相同则保留先出现的候选。
/// 不应用置信度阈值或领域标签，调用方根据分数决定是否接受结果。
/// 模板加载或匹配失败通过 `Err` 传播，不将错误解释为候选不存在。
#[allow(dead_code, reason = "通用识别 primitive，供后续工作流调用")]
pub(crate) fn find_best<'a, I, P>(
    search: &ImageRegion<'_, I>,
    template_names: &[&'a str],
    templates: &mut P,
) -> anyhow::Result<Option<(&'a str, MatchResult)>>
where
    I: GenericImageView,
    I::Pixel: Pixel<Subpixel = u8>,
    P: TemplateProvider + ?Sized,
{
    let mut best: Option<(&str, MatchResult)> = None;
    for &name in template_names {
        let matched = find(search, name, templates)?;
        if best
            .as_ref()
            .is_none_or(|(_, current)| matched.score > current.score)
        {
            best = Some((name, matched));
        }
    }
    Ok(best)
}

/// 使用已加载模板的纯计算接口。
pub(crate) mod pure {
    use anyhow::bail;
    use image::{GenericImageView, Pixel, imageops};
    use imageproc::template_matching;

    use super::{super::ccoeff, ImageRegion, MatchResult, Region2D};

    /// 使用已加载的模板在已校验的区域内搜索。
    ///
    /// 结果区域始终相对于原图，而非区域视图。
    pub(crate) fn match_template<I, T>(
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
    pub(super) fn match_in_region<I, T>(
        image_region: &I,
        template: &T,
    ) -> anyhow::Result<MatchResult>
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
    use image::{Rgb, RgbImage};

    use super::*;

    struct Templates {
        exact: RgbImage,
        inverse: RgbImage,
    }

    impl TemplateProvider for Templates {
        fn get(&mut self, name: &str) -> anyhow::Result<&RgbImage> {
            match name {
                "exact" | "same" => Ok(&self.exact),
                "inverse" => Ok(&self.inverse),
                _ => anyhow::bail!("missing template: {name}"),
            }
        }
    }

    fn templates() -> Templates {
        let exact = RgbImage::from_fn(2, 2, |x, y| Rgb([30 + (x + y * 2) as u8 * 50; 3]));
        let inverse = RgbImage::from_fn(2, 2, |x, y| Rgb(exact.get_pixel(x, y).0.map(|v| 255 - v)));
        Templates { exact, inverse }
    }

    #[test]
    fn selects_the_best_candidate_and_preserves_image_coordinates() {
        let mut templates = templates();
        let mut image = RgbImage::new(6, 5);
        image::imageops::replace(&mut image, &templates.exact, 3, 2);
        let region = ImageRegion::new(&image, Region2D::from_ltwh(2, 1, 4, 4)).unwrap();
        let (name, matched) = find_best(&region, &["inverse", "exact", "same"], &mut templates)
            .unwrap()
            .unwrap();

        assert_eq!(name, "exact");
        assert_eq!(matched.region, Region2D::from_ltwh(3, 2, 2, 2));
        assert!((matched.score - 1.0).abs() < 0.001);
    }

    #[test]
    fn keeps_a_negative_score_for_the_caller_to_interpret() {
        let mut templates = templates();
        let image = templates.exact.clone();
        let region = ImageRegion::new(&image, Region2D::from_ltwh(0, 0, 2, 2)).unwrap();
        let (_, matched) = find_best(&region, &["inverse"], &mut templates)
            .unwrap()
            .unwrap();
        assert!((matched.score + 1.0).abs() < 0.001);
    }

    #[test]
    fn distinguishes_empty_candidates_from_loading_errors() {
        let mut templates = templates();
        let image = templates.exact.clone();
        let region = ImageRegion::new(&image, Region2D::from_ltwh(0, 0, 2, 2)).unwrap();
        assert_eq!(find_best(&region, &[], &mut templates).unwrap(), None);
        assert!(find_best(&region, &["exact", "missing"], &mut templates).is_err());
    }

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
