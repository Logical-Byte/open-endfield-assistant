use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use image::{GenericImageView, Pixel, RgbImage};

use super::MatchResult;
use crate::utils::region::Region2D;

/// 模板管理器，管理一个文件夹下的所有模板图片，按需懒加载并缓存。
pub struct TemplateManager {
    folder: PathBuf,
    cache: HashMap<String, RgbImage>,
}

impl TemplateManager {
    /// 创建 [`TemplateManager`]，`folder` 为模板图片所在的文件夹路径。
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
            cache: HashMap::new(),
        }
    }

    /// 加载指定名称的模板图片（仅首次使用时加载）。
    fn load_template(&mut self, template_name: &str) -> Result<&RgbImage> {
        let path = self.folder.join(template_name);
        // 加载失败时附加完整路径，便于上层（场景识别）定位是哪个模板缺失
        let image = image::open(&path)
            .with_context(|| format!("加载模板图片失败: {}", path.display()))?
            .to_rgb8();
        self.cache.insert(template_name.to_string(), image);
        Ok(self.cache.get(template_name).unwrap())
    }

    /// 确保模板已加载，返回模板图像的引用。
    fn ensure_template(&mut self, template_name: &str) -> Result<&RgbImage> {
        // 不能这么写，否则会出现借用检查器错误
        //
        // if let Some(image) = self.cache.get(template_name) {
        //     Ok(image)
        // } else {
        //     self.load_template(template_name)
        // }

        if self.cache.contains_key(template_name) {
            Ok(self.cache.get(template_name).unwrap())
        } else {
            self.load_template(template_name)
        }
    }

    /// 在 `image` 的 `search_region` 区域内搜索名为 `template_name` 的模板。
    ///
    /// 模板会在第一次使用时自动从 `folder` 文件夹中加载并缓存。
    pub fn match_template_in_region<I>(
        &mut self,
        image: &I,
        template_name: &str,
        search_region: Option<Region2D<u32>>,
    ) -> Result<MatchResult>
    where
        I: GenericImageView,
        I::Pixel: Pixel<Subpixel = u8>,
    {
        let template = self.ensure_template(template_name)?;
        super::match_template_in_region(image, template, search_region)
    }
}
