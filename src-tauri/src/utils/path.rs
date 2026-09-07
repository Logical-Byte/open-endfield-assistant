use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// 解析并验证当前存在于 `root` 内的相对文件路径。
pub(crate) fn resolve_existing_relative_file(root: &Path, relative_path: &str) -> Result<PathBuf> {
    if relative_path.is_empty() {
        bail!("相对文件路径不能为空");
    }

    let mut path = root.to_path_buf();
    for segment in relative_path.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains(':')
        {
            bail!("相对文件路径包含无效片段: {relative_path:?}");
        }
        path.push(segment);
    }

    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("规范化根目录失败: {}", root.display()))?;
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("规范化文件路径失败: {}", path.display()))?;

    if !canonical_path.starts_with(&canonical_root) {
        bail!("文件路径超出根目录: {relative_path:?}");
    }
    if !canonical_path.is_file() {
        bail!("路径不是文件: {}", canonical_path.display());
    }

    Ok(canonical_path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::resolve_existing_relative_file;

    #[test]
    fn resolves_an_existing_nested_file_within_root() {
        let root = tempfile::tempdir().unwrap();
        let nested_dir = root.path().join("nested");
        fs::create_dir(&nested_dir).unwrap();
        let file = nested_dir.join("resource.bin");
        fs::write(&file, b"resource").unwrap();

        assert_eq!(
            resolve_existing_relative_file(root.path(), "nested/resource.bin").unwrap(),
            file.canonicalize().unwrap()
        );
    }

    #[test]
    fn rejects_relative_paths_that_are_empty_or_not_normalized() {
        let root = tempfile::tempdir().unwrap();
        let nested_dir = root.path().join("nested");
        fs::create_dir(&nested_dir).unwrap();
        fs::write(root.path().join("resource.bin"), b"resource").unwrap();
        fs::write(nested_dir.join("resource.bin"), b"resource").unwrap();

        for relative_path in [
            "",
            "/resource.bin",
            "../resource.bin",
            "nested/../resource.bin",
            "./resource.bin",
            "nested//resource.bin",
            "nested/",
            r"nested\resource.bin",
            "resource.bin:stream",
        ] {
            assert!(
                resolve_existing_relative_file(root.path(), relative_path).is_err(),
                "unexpectedly accepted {relative_path:?}"
            );
        }
    }

    #[test]
    fn rejects_missing_files_and_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("directory")).unwrap();
        let missing_root = root.path().join("missing-root");

        assert!(resolve_existing_relative_file(&missing_root, "resource.bin").is_err());
        assert!(resolve_existing_relative_file(root.path(), "missing.bin").is_err());
        assert!(resolve_existing_relative_file(root.path(), "directory").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn allows_internal_symlinks_and_rejects_escaping_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let internal_file = root.path().join("internal.bin");
        let outside_file = outside.path().join("outside.bin");
        fs::write(&internal_file, b"internal").unwrap();
        fs::write(&outside_file, b"outside").unwrap();
        symlink(&internal_file, root.path().join("internal-link.bin")).unwrap();
        symlink(&outside_file, root.path().join("escaping-link.bin")).unwrap();

        assert_eq!(
            resolve_existing_relative_file(root.path(), "internal-link.bin").unwrap(),
            internal_file.canonicalize().unwrap()
        );
        assert!(resolve_existing_relative_file(root.path(), "escaping-link.bin").is_err());
    }
}
