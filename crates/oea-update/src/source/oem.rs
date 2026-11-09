//! OEM 下载清单的请求与校验。
use super::{
    AvailableUpdateMetadata, DownloadPlan, default_filename, read_body_with_cancellation,
    send_with_cancellation,
};
use crate::{Error, Reason, http, response};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::debug;

const OEM_STABLE_MANIFEST_URL: &str = "https://package.oem.re/channels/oea/stable.json";

#[derive(Debug, Deserialize)]
struct OemManifest {
    version: String,
    tag: String,
    filename: String,
    url: String,
    size: u64,
    sha256: String,
}

pub(super) async fn resolve(
    metadata: &AvailableUpdateMetadata,
    proxy_url: Option<&str>,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, Error> {
    let client = http::build_client(proxy_url, user_agent)?;
    let response = send_with_cancellation(
        client
            .get(OEM_STABLE_MANIFEST_URL)
            .header(reqwest::header::ACCEPT, "application/json"),
        cancellation,
        "OEM 更新元数据请求失败",
    )
    .await?;
    if !response.status().is_success() {
        return Err(Error::failed(
            Reason::Network,
            anyhow::anyhow!(
                "OEM 更新元数据请求失败（HTTP {}），已中断下载",
                response.status()
            ),
        ));
    }
    let body =
        read_body_with_cancellation(response, cancellation, "OEM 更新元数据响应读取失败").await?;
    let manifest: OemManifest = serde_json::from_slice(&body).map_err(|error| {
        Error::failed(
            Reason::InvalidMetadata,
            anyhow::Error::new(error).context("OEM 更新元数据不是有效 JSON，已中断下载"),
        )
    })?;
    parse_oem_manifest(&metadata.version_name, manifest)
}

fn parse_oem_manifest(
    expected_version: &str,
    manifest: OemManifest,
) -> Result<DownloadPlan, Error> {
    let normalized_expected = expected_version
        .trim()
        .strip_prefix(['v', 'V'])
        .unwrap_or(expected_version.trim());
    if manifest.version != normalized_expected {
        return Err(Error::failed(
            Reason::VersionMismatch {
                expected: expected_version.to_string(),
                actual: manifest.tag.clone(),
            },
            anyhow::anyhow!(
                "OEM 与 Mirror酱版本不一致（OEM: {}，Mirror酱: {expected_version}），已中断下载",
                manifest.tag
            ),
        ));
    }
    debug!(
        source = "oem",
        version = %manifest.tag,
        filename = %manifest.filename,
        expected_bytes = manifest.size,
        "OEM 更新清单与检查结果版本一致"
    );
    Ok(DownloadPlan {
        url: manifest.url,
        filename: response::sanitize_filename(&manifest.filename)
            .unwrap_or_else(|| default_filename(expected_version)),
        expected_sha256: Some(manifest.sha256),
        total_size: Some(manifest.size),
        source_label: "OEM",
        proxy_url: None,
        accept: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oem_manifest_requires_the_cached_version() {
        let manifest = || OemManifest {
            version: "1.3.0".to_string(),
            tag: "v1.3.0".to_string(),
            filename: "oea.zip".to_string(),
            url: "https://example.com/oea.zip".to_string(),
            size: 42,
            sha256: "abc".to_string(),
        };
        assert_eq!(
            parse_oem_manifest("v1.3.0", manifest())
                .unwrap()
                .source_label,
            "OEM"
        );
        assert!(parse_oem_manifest("v1.4.0", manifest()).is_err());
    }
}
