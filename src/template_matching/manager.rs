use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use image::RgbImage;

use super::MatchResult;
use crate::utils::region::Region2D;

// pub struct ImageTemplate<'a> {
//     path: &'a Path,
//     image: Option<RgbImage>,
// }

// impl<'a> ImageTemplate<'a> {
//     pub fn new<P>(path: &'a P) -> Self
//     where
//         P: AsRef<Path> + ?Sized,
//     {
//         Self {
//             path: path.as_ref(),
//             image: None,
//         }
//     }

//     pub fn load_template(&mut self) -> Result<&RgbImage> {
//         let image = image::open(self.path)?.to_rgb8();
//         self.image = Some(image);
//         Ok(self.image.as_ref().unwrap())
//     }

//     pub fn ensure_template(&mut self) -> Result<&RgbImage> {
//         if self.image.is_none() {
//             self.load_template()?;
//         }
//         Ok(self.image.as_ref().unwrap())
//     }

//     pub fn match_template_in_region(
//         &mut self,
//         image: &RgbImage,
//         search_region: Option<Region2D<u32>>,
//     ) -> Result<MatchResult> {
//         let template = self.ensure_template()?;
//         super::match_template_in_region(image, template, search_region)
//     }
// }

/// 模板管理器，管理一个文件夹下的所有模板图片，按需懒加载并缓存。
pub struct TemplateManager {
    folder: PathBuf,
    cache: HashMap<String, RgbImage>,
}

impl TemplateManager {
    /// 创建 `TemplateManager`，`folder` 为模板图片所在的文件夹路径。
    pub fn new<P>(folder: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            folder: folder.as_ref().to_path_buf(),
            cache: HashMap::new(),
        }
    }

    /// 加载指定名称的模板图片（仅首次使用时加载）。
    fn load_template(&mut self, template_name: &str) -> Result<&RgbImage> {
        let path = self.folder.join(template_name);
        let image = image::open(&path)?.to_rgb8();
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
    pub fn match_template_in_region(
        &mut self,
        image: &RgbImage,
        template_name: &str,
        search_region: Option<Region2D<u32>>,
    ) -> Result<MatchResult> {
        let template = self.ensure_template(template_name)?;
        super::match_template_in_region(image, template, search_region)
    }
}
