//! 在单个固定识别帧上提供场景识别能力。

use anyhow::Result;
use image::{RgbaImage, imageops};

use crate::{
    template_matching::{
        LazyTemplateLoader, MatchResult, TemplateSource, match_template_in_region,
    },
    utils::region::Region2D,
};

/// [`Session`](super::Session) 所有识别基础设施的受限视图。
///
/// 识别帧在上下文存续期内保持不变。模板仍按需加载，并可能修改会话所有的模板缓存。
pub struct RecognitionContext<'a> {
    frame: &'a RgbaImage,
    templates: &'a mut LazyTemplateLoader,
}

impl<'a> RecognitionContext<'a> {
    pub(super) fn new(frame: &'a RgbaImage, templates: &'a mut LazyTemplateLoader) -> Self {
        Self { frame, templates }
    }

    /// 返回正在分类的 1280×720 识别帧。
    pub fn frame(&self) -> &RgbaImage {
        self.frame
    }

    /// 在识别帧的指定区域内搜索模板。
    pub fn find_template_in_roi(
        &mut self,
        template_name: &str,
        roi: Region2D<u32>,
        threshold: f32,
    ) -> Result<Option<MatchResult>> {
        let template = self.templates.get(template_name)?;
        let result = match_template_in_region(self.frame, template, Some(roi))?;
        if result.score >= threshold {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// 判断矩形区域的平均灰度是否低于 `threshold`。
    pub fn is_roi_dark_ltwh(&self, x: u32, y: u32, width: u32, height: u32, threshold: u8) -> bool {
        let roi = Region2D::from_ltwh(x, y, width, height);
        let cropped = imageops::crop_imm(self.frame, roi.x0(), roi.y0(), roi.width(), roi.height());
        let gray = imageops::grayscale(&cropped.to_image());
        let pixel_count = (roi.width() * roi.height()) as u64;
        if pixel_count == 0 {
            return false;
        }

        let total: u64 = gray.pixels().map(|pixel| pixel.0[0] as u64).sum();
        ((total / pixel_count) as u8) < threshold
    }
}
