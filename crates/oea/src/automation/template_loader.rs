//! OEA 模板资源的相对路径解析、懒加载和缓存。

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use image::RgbImage;

use crate::utils::path::resolve_existing_relative_file;

/// 从模板根目录按需加载图片，并缓存成功加载的结果。
///
/// 缓存结果在加载器的生命周期内保持有效；加载失败不会进入缓存，后续调用会重试。
pub struct TemplateLoader {
    root: PathBuf,
    cache: HashMap<String, RgbImage>,
}

impl TemplateLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: HashMap::new(),
        }
    }

    /// 按根目录下的相对文件名取模板，首次读取成功后缓存 RGB 图片。
    pub fn get(&mut self, template_name: &str) -> anyhow::Result<&RgbImage> {
        if self.cache.contains_key(template_name) {
            return Ok(self
                .cache
                .get(template_name)
                .expect("已缓存的模板应存在于缓存中"));
        }

        let path = resolve_existing_relative_file(&self.root, template_name)
            .with_context(|| format!("解析模板文件失败: {template_name:?}"))?;
        let image = image::open(&path)
            .with_context(|| format!("加载模板图片失败: {}", path.display()))?
            .to_rgb8();
        self.cache.insert(template_name.to_owned(), image);

        Ok(self
            .cache
            .get(template_name)
            .expect("模板应在加载后存在于缓存中"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn returns_a_cached_template_after_its_file_is_removed() {
        use image::{Rgb, RgbImage};

        use super::TemplateLoader;

        let templates = tempfile::tempdir().unwrap();
        let template = templates.path().join("cached.png");
        RgbImage::from_pixel(1, 1, Rgb([0, 0, 0]))
            .save(&template)
            .unwrap();

        let mut loader = TemplateLoader::new(templates.path());
        loader.get("cached.png").unwrap();
        fs::remove_file(template).unwrap();

        assert_eq!(loader.get("cached.png").unwrap().width(), 1);
    }
}
