use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use image::RgbImage;

use crate::utils::path::resolve_existing_relative_file;

/// 按逻辑名称提供模板图片，不向调用方暴露存储与缓存策略。
pub(crate) trait TemplateSource {
    /// 返回由模板源持有的图片。
    fn get(&mut self, template_name: &str) -> Result<&RgbImage>;
}

/// 从模板根目录按需加载图片，并缓存成功加载的结果。
///
/// 缓存结果在加载器的生命周期内保持有效；加载失败不会进入缓存，后续调用会重试。
pub(crate) struct LazyTemplateLoader {
    root: PathBuf,
    cache: HashMap<String, RgbImage>,
}

impl LazyTemplateLoader {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: HashMap::new(),
        }
    }
}

impl TemplateSource for LazyTemplateLoader {
    fn get(&mut self, template_name: &str) -> Result<&RgbImage> {
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

        use super::{LazyTemplateLoader, TemplateSource};

        let templates = tempfile::tempdir().unwrap();
        let template = templates.path().join("cached.png");
        RgbImage::from_pixel(1, 1, Rgb([0, 0, 0]))
            .save(&template)
            .unwrap();

        let mut loader = LazyTemplateLoader::new(templates.path());
        loader.get("cached.png").unwrap();
        fs::remove_file(template).unwrap();

        assert_eq!(loader.get("cached.png").unwrap().width(), 1);
    }
}
