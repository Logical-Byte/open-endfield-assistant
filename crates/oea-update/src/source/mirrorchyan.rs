//! 复用检查请求取得的 Mirror酱下载信息。
use super::{AvailableUpdateMetadata, DownloadPlan, default_filename};
use crate::{Error, Reason};

pub(super) fn resolve(
    metadata: &AvailableUpdateMetadata,
    cdk: &str,
) -> Result<DownloadPlan, Error> {
    if cdk.trim().is_empty() {
        return Err(Error::failed(
            Reason::InvalidMetadata,
            anyhow::anyhow!("Mirror酱下载需要提供 CDK"),
        ));
    }
    let package = metadata.mirrorchyan_package.as_ref().ok_or_else(|| {
        Error::failed(
            Reason::PackageUnavailable {
                version: metadata.version_name.clone(),
            },
            anyhow::anyhow!("检查结果中没有 Mirror酱安装包"),
        )
    })?;
    Ok(DownloadPlan {
        url: package.url.clone(),
        filename: default_filename(&metadata.version_name),
        expected_sha256: package.expected_sha256.clone(),
        total_size: package.file_size,
        source_label: "Mirror酱",
        proxy_url: None,
        accept: None,
    })
}
