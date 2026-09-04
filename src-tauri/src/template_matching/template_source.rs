use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use image::RgbImage;

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
        let path = resolve_template_path(&self.root, template_name)?;

        if self.cache.contains_key(template_name) {
            return Ok(self
                .cache
                .get(template_name)
                .expect("已缓存的模板应存在于缓存中"));
        }

        let canonical_root = self
            .root
            .canonicalize()
            .with_context(|| format!("规范化模板根目录失败: {}", self.root.display()))?;
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("规范化模板路径失败: {}", path.display()))?;
        if !canonical_path.starts_with(&canonical_root) {
            bail!("模板路径超出模板根目录: {template_name:?}");
        }

        let image = image::open(&canonical_path)
            .with_context(|| format!("加载模板图片失败: {}", canonical_path.display()))?
            .to_rgb8();
        self.cache.insert(template_name.to_owned(), image);

        Ok(self
            .cache
            .get(template_name)
            .expect("模板应在加载后存在于缓存中"))
    }
}

/// 将使用 `/` 分隔的模板名称解析到 `root` 内。
///
/// 逐段构造路径，使校验规则在开发环境与 Windows 运行环境中保持一致。
fn resolve_template_path(root: &Path, template_name: &str) -> Result<PathBuf> {
    if template_name.is_empty() {
        bail!("模板名称不能为空");
    }

    let mut path = root.to_path_buf();
    for segment in template_name.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains(':')
        {
            bail!("模板名称包含无效路径片段: {template_name:?}");
        }
        path.push(segment);
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::resolve_template_path;

    #[test]
    fn resolves_valid_template_names_within_root() {
        let root = Path::new("templates");

        assert_eq!(
            resolve_template_path(root, "档案库.png").unwrap(),
            root.join("档案库.png")
        );
        assert_eq!(
            resolve_template_path(root, "情报档案库/下一篇.png").unwrap(),
            root.join("情报档案库").join("下一篇.png")
        );
    }

    #[test]
    fn rejects_template_names_that_can_escape_or_are_not_normalized() {
        for template_name in [
            "",
            "/档案库.png",
            "../档案库.png",
            "情报档案库/../档案库.png",
            "./档案库.png",
            "情报档案库//档案库.png",
            "情报档案库/",
            r"C:\档案库.png",
            r"\\server\share\档案库.png",
            "档案库.png:stream",
        ] {
            assert!(
                resolve_template_path(Path::new("templates"), template_name).is_err(),
                "unexpectedly accepted {template_name:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_that_resolves_outside_template_root() {
        use std::os::unix::fs::symlink;

        use image::{Rgb, RgbImage};

        use super::{LazyTemplateLoader, TemplateSource};

        let templates = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_template = outside.path().join("outside.png");
        RgbImage::from_pixel(1, 1, Rgb([0, 0, 0]))
            .save(&outside_template)
            .unwrap();
        symlink(&outside_template, templates.path().join("escaped.png")).unwrap();

        let mut loader = LazyTemplateLoader::new(templates.path());
        assert!(loader.get("escaped.png").is_err());
    }
}
