//! 完整发布包的安全解压与版本资源发布。
//!
//! ZIP 先解压到事务内的候选目录。目标版本资源尚未发布时，只有根入口、`models/`
//! 和 `resources/` 都存在后，才会把整个版本资源目录一次性移动到便携根目录。
//! 目标目录已存在时按 Issue #21 的第一阶段规则直接复用，不重新检查内容。

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use super::transaction::{ARTIFACT, CANDIDATE_EXE};

/// 将完整发布包装入事务候选区，并一次性发布目标版本资源目录。
///
/// 返回候选 `OEA.exe` 的路径；更新工作流随后复制 Bootstrap 并准备平台交接参数。
pub fn prepare_full_package(
    portable_root: &Path,
    transaction_dir: &Path,
    target_version: &str,
) -> Result<PathBuf> {
    // 候选区是可重建的临时状态，每次准备都从空目录开始。
    let candidate_dir = transaction_dir.join("candidate");
    if candidate_dir.exists() {
        fs::remove_dir_all(&candidate_dir).context("重置候选目录失败")?;
    }
    fs::create_dir_all(&candidate_dir).context("创建候选目录失败")?;

    let archive_file =
        fs::File::open(transaction_dir.join(ARTIFACT)).context("打开更新归档失败")?;
    let mut archive = zip::ZipArchive::new(archive_file).context("读取更新归档失败")?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("读取归档条目失败")?;
        // `enclosed_name` 拒绝 `../`、绝对路径等会逃出候选目录的 ZIP 条目。
        let enclosed = entry.enclosed_name().context("归档包含危险路径")?;
        let destination = candidate_dir.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&destination).context("创建候选目录失败")?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).context("创建候选文件父目录失败")?;
        }
        let mut output = fs::File::create(&destination).context("创建候选文件失败")?;
        io::copy(&mut entry, &mut output).context("解压候选文件失败")?;
    }

    // 完整包的顶层布局必须与发布脚本生成的便携 ZIP 完全一致。
    let candidate_exe = transaction_dir.join(CANDIDATE_EXE);
    if !candidate_exe.is_file() {
        bail!("完整包缺少根入口 OEA.exe");
    }

    let relative_assets = PathBuf::from("assets").join(format!("v{target_version}"));
    let candidate_assets = candidate_dir.join(&relative_assets);
    let published_assets = portable_root.join(&relative_assets);
    if !published_assets.exists() {
        if !candidate_assets.join("models").is_dir() || !candidate_assets.join("resources").is_dir()
        {
            bail!("完整包缺少目标版本的 models 或 resources 目录");
        }
        if let Some(parent) = published_assets.parent() {
            fs::create_dir_all(parent).context("创建版本资源父目录失败")?;
        }
        // 同一文件系统内的目录 `rename` 对观察者是整体切换，不会暴露半套资源。
        fs::rename(&candidate_assets, &published_assets).context("发布版本资源目录失败")?;
    }
    // 已存在的版本目录按 Issue #21 第一阶段规格直接复用，不验证其来源或内容。

    Ok(candidate_exe)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use super::prepare_full_package;

    fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn valid_full_package_publishes_only_complete_version_assets() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(&transaction).unwrap();
        let archive = transaction.join("artifact.zip");
        write_zip(
            &archive,
            &[
                ("OEA.exe", b"new exe"),
                ("assets/v0.2.0/models/model.onnx", b"model"),
                ("assets/v0.2.0/resources/data.json", b"data"),
            ],
        );

        let candidate = prepare_full_package(&root, &transaction, "0.2.0").unwrap();

        assert_eq!(fs::read(candidate).unwrap(), b"new exe");
        assert_eq!(
            fs::read(root.join("assets/v0.2.0/resources/data.json")).unwrap(),
            b"data"
        );
    }

    #[test]
    fn unsafe_package_never_publishes_assets() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(&transaction).unwrap();
        write_zip(
            &transaction.join("artifact.zip"),
            &[("OEA.exe", b"new exe"), ("../escape", b"bad")],
        );

        assert!(prepare_full_package(&root, &transaction, "0.2.0").is_err());
        assert!(!root.join("assets/v0.2.0").exists());
    }

    #[test]
    fn incomplete_package_never_publishes_assets() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(&transaction).unwrap();
        write_zip(
            &transaction.join("artifact.zip"),
            &[
                ("OEA.exe", b"new exe"),
                ("assets/v0.2.0/resources/data.json", b"data"),
            ],
        );

        assert!(prepare_full_package(&root, &transaction, "0.2.0").is_err());
        assert!(!root.join("assets/v0.2.0").exists());
    }

    #[test]
    fn existing_version_assets_are_reused_without_rechecking_contents() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(root.join("assets/v0.2.0")).unwrap();
        fs::create_dir_all(&transaction).unwrap();
        write_zip(
            &transaction.join("artifact.zip"),
            &[("OEA.exe", b"new exe")],
        );

        let candidate = prepare_full_package(&root, &transaction, "0.2.0").unwrap();

        assert_eq!(fs::read(candidate).unwrap(), b"new exe");
    }
}
