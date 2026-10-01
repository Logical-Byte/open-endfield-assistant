use anyhow::{Context, Result, bail};
use std::{
    fs::OpenOptions,
    io::Cursor,
    path::{Path, PathBuf},
};

pub(super) fn save(image: &image::RgbaImage, output: Option<&Path>, cwd: &Path) -> Result<PathBuf> {
    let path = output
        .map(|path| cwd.join(path))
        .unwrap_or_else(|| cwd.to_owned());
    let path = if path.is_dir() {
        path.join(format!(
            "oea-screenshot-{}.png",
            chrono::Local::now().format("%Y%m%d-%H%M%S-%3f")
        ))
    } else {
        if path.extension().is_some_and(|extension| extension != "png") {
            bail!(
                "screenshot output must have .png or no extension, got: {}",
                path.display()
            );
        }
        path
    };
    // Resolve the parent before opening the destination. Never create directories,
    // and let `create_new` atomically reject existing files and naming collisions.
    let parent = path
        .parent()
        .with_context(|| format!("screenshot output requires a parent directory: {}", path.display()))?
        .canonicalize()
        .with_context(|| format!("failed to resolve parent directory for screenshot output {} (the directory must already exist)", path.display()))?;
    let path =
        parent.join(path.file_name().with_context(|| {
            format!("screenshot output requires a filename: {}", path.display())
        })?);
    write_png(image, &path)?;
    Ok(path)
}

fn write_png(image: &image::RgbaImage, path: &Path) -> Result<()> {
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .with_context(|| {
            format!(
                "failed to encode {}x{} screenshot as PNG for {}",
                image.width(),
                image.height(),
                path.display()
            )
        })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "failed to create screenshot file {} (existing files are never overwritten)",
                path.display()
            )
        })?;
    std::io::Write::write_all(&mut file, png.get_ref())
        .with_context(|| format!("failed to write screenshot PNG: {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn saves_png_without_overwriting_and_respects_output_path_rules() {
        let dir = tempfile::tempdir().unwrap();
        let image = image::RgbaImage::new(8, 6);
        let exact = dir.path().join("asset");
        let saved = super::save(&image, Some(&exact), dir.path()).unwrap();
        assert!(saved.is_absolute());
        let bytes = std::fs::read(&saved).unwrap();
        assert_eq!(
            image::load_from_memory(&bytes)
                .unwrap()
                .to_rgba8()
                .dimensions(),
            (8, 6)
        );
        assert!(super::save(&image, Some(&exact), dir.path()).is_err());
        assert_eq!(std::fs::read(saved).unwrap(), bytes);
        for invalid in [
            dir.path().join("missing/asset.png"),
            dir.path().join("asset.jpg"),
        ] {
            assert!(super::save(&image, Some(&invalid), dir.path()).is_err());
            assert!(!invalid.exists());
        }
        let default = super::save(&image, None, dir.path()).unwrap();
        let subdir = dir.path().join("captures");
        std::fs::create_dir(&subdir).unwrap();
        let directory = super::save(&image, Some(&subdir), dir.path()).unwrap();
        assert_eq!(directory.parent().unwrap(), subdir.canonicalize().unwrap());
        let relative = super::save(
            &image,
            Some(std::path::Path::new("relative.png")),
            dir.path(),
        )
        .unwrap();
        assert_eq!(
            relative.parent().unwrap(),
            dir.path().canonicalize().unwrap()
        );
        for path in [default, directory] {
            assert_eq!(path.extension().unwrap(), "png");
        }
    }
}
