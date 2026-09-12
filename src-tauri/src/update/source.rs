//! 将已缓存的可用更新解析为一次下载所需的私有计划。

use reqwest::StatusCode;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::{OeaConfig, UpdateSource};

use super::{
    check::{self, AvailableUpdateMetadata, MirrorchyanPackage},
    http, response,
};

const OEM_STABLE_MANIFEST_URL: &str = "https://package.oem.re/channels/oea/stable.json";
const GITHUB_RELEASE_TAG_URL: &str =
    "https://api.github.com/repos/Logical-Byte/open-endfield-assistant/releases/tags/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DownloadPlan {
    pub(super) url: String,
    pub(super) filename: String,
    pub(super) expected_sha256: Option<String>,
    pub(super) total_size: Option<u64>,
    pub(super) source: UpdateSource,
    pub(super) accept: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceChoice {
    Mirrorchyan,
    Oem,
    Github,
}

#[derive(Debug, Deserialize)]
struct OemManifest {
    version: String,
    tag: String,
    filename: String,
    url: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    url: String,
    size: u64,
    digest: Option<String>,
}

pub(super) async fn resolve_download_plan(
    metadata: &AvailableUpdateMetadata,
    config: &OeaConfig,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, String> {
    let cdk = check::configured_cdk(config);
    match select_source(
        config.update_source,
        cdk.as_deref(),
        metadata.mirrorchyan_package.as_ref(),
    ) {
        SourceChoice::Mirrorchyan => Ok(mirrorchyan_plan(
            metadata,
            metadata
                .mirrorchyan_package
                .as_ref()
                .expect("选择 MirrorChyan 时必须有缓存下载信息"),
        )),
        SourceChoice::Oem => resolve_oem_plan(metadata, config, user_agent, cancellation).await,
        SourceChoice::Github => {
            resolve_github_plan(metadata, config, user_agent, cancellation).await
        }
    }
}

fn select_source(
    configured: UpdateSource,
    cdk: Option<&str>,
    package: Option<&MirrorchyanPackage>,
) -> SourceChoice {
    match configured {
        UpdateSource::Mirrorchyan
            if cdk.is_some_and(|value| !value.trim().is_empty()) && package.is_some() =>
        {
            SourceChoice::Mirrorchyan
        }
        UpdateSource::Mirrorchyan | UpdateSource::Oem => SourceChoice::Oem,
        UpdateSource::Github => SourceChoice::Github,
    }
}

fn mirrorchyan_plan(
    metadata: &AvailableUpdateMetadata,
    package: &MirrorchyanPackage,
) -> DownloadPlan {
    DownloadPlan {
        url: package.url.clone(),
        filename: default_filename(&metadata.version_name),
        expected_sha256: package.expected_sha256.clone(),
        total_size: package.file_size,
        source: UpdateSource::Mirrorchyan,
        accept: None,
    }
}

async fn resolve_oem_plan(
    metadata: &AvailableUpdateMetadata,
    config: &OeaConfig,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, String> {
    let client = http::build_client(config, user_agent)?;
    let response = send_with_cancellation(
        client
            .get(OEM_STABLE_MANIFEST_URL)
            .header(reqwest::header::ACCEPT, "application/json"),
        cancellation,
        "OEM 更新元数据请求失败",
    )
    .await?;
    if !response.status().is_success() {
        return Err(format!(
            "OEM 更新元数据请求失败（HTTP {}），已中断下载",
            response.status()
        ));
    }
    let body =
        read_body_with_cancellation(response, cancellation, "OEM 更新元数据响应读取失败").await?;
    let manifest: OemManifest = serde_json::from_slice(&body)
        .map_err(|error| format!("OEM 更新元数据不是有效 JSON，已中断下载: {error}"))?;
    parse_oem_manifest(&metadata.version_name, manifest)
}

fn parse_oem_manifest(
    expected_version: &str,
    manifest: OemManifest,
) -> Result<DownloadPlan, String> {
    let normalized_expected = expected_version
        .trim()
        .strip_prefix(['v', 'V'])
        .unwrap_or(expected_version.trim());
    if manifest.version != normalized_expected {
        return Err(format!(
            "OEM 与 Mirror酱版本不一致（OEM: {}，Mirror酱: {expected_version}），已中断下载",
            manifest.tag
        ));
    }
    info!(
        "OEM 与 Mirror酱版本一致: {}，下载地址: {}",
        manifest.tag, manifest.url
    );
    Ok(DownloadPlan {
        url: manifest.url,
        filename: response::sanitize_filename(&manifest.filename)
            .unwrap_or_else(|| default_filename(expected_version)),
        expected_sha256: Some(manifest.sha256),
        total_size: Some(manifest.size),
        source: UpdateSource::Oem,
        accept: None,
    })
}

async fn resolve_github_plan(
    metadata: &AvailableUpdateMetadata,
    config: &OeaConfig,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, String> {
    let client = http::build_client(config, user_agent)?;
    let mut url = reqwest::Url::parse(GITHUB_RELEASE_TAG_URL)
        .map_err(|error| format!("GitHub API URL 构造失败: {error}"))?;
    url.path_segments_mut()
        .map_err(|_| "GitHub API URL 无法追加版本号".to_string())?
        .pop_if_empty()
        .push(&metadata.version_name);
    let response = send_with_cancellation(
        client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json"),
        cancellation,
        "GitHub API 请求失败",
    )
    .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(format!(
            "GitHub 上未找到版本 {} 的 Release",
            metadata.version_name
        ));
    }
    if !response.status().is_success() {
        return Err(format!(
            "GitHub API 错误（HTTP {}），GitHub 下载暂不可用",
            response.status()
        ));
    }
    let body =
        read_body_with_cancellation(response, cancellation, "GitHub API 响应读取失败").await?;
    let release: GithubRelease = serde_json::from_slice(&body)
        .map_err(|error| format!("GitHub API 响应不是有效 JSON: {error}"))?;
    select_github_asset(&metadata.version_name, release)
}

fn select_github_asset(version_name: &str, release: GithubRelease) -> Result<DownloadPlan, String> {
    let exact_name = default_filename(version_name);
    let mut candidates = release
        .assets
        .into_iter()
        .filter(|asset| asset.name.to_ascii_lowercase().ends_with(".zip"))
        .collect::<Vec<_>>();
    let exact_index = candidates.iter().position(|asset| asset.name == exact_name);
    let asset = exact_index
        .map(|index| candidates.swap_remove(index))
        .or_else(|| candidates.into_iter().max_by_key(|asset| asset.size))
        .ok_or_else(|| "GitHub Release 中未找到 OEA-windows-x86_64 的 zip 资产".to_string())?;

    let expected_sha256 = parse_github_digest(asset.digest.as_deref());
    if expected_sha256.is_none() {
        warn!(
            "GitHub 资产缺少合法的 sha256 digest，跳过 sha256 校验: {}",
            asset.name
        );
    }
    Ok(DownloadPlan {
        url: asset.url,
        filename: response::sanitize_filename(&asset.name)
            .unwrap_or_else(|| default_filename(version_name)),
        expected_sha256,
        total_size: Some(asset.size),
        source: UpdateSource::Github,
        accept: Some("application/octet-stream"),
    })
}

fn parse_github_digest(digest: Option<&str>) -> Option<String> {
    let (algorithm, digest) = digest?.split_once(':')?;
    if !algorithm.eq_ignore_ascii_case("sha256") {
        return None;
    }
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| digest.to_ascii_lowercase())
}

pub(super) fn default_filename(version_name: &str) -> String {
    format!("OEA-windows-x86_64-{version_name}.zip")
}

async fn send_with_cancellation(
    request: reqwest::RequestBuilder,
    cancellation: &CancellationToken,
    context: &str,
) -> Result<reqwest::Response, String> {
    tokio::select! {
        biased;
        response = request.send() => response.map_err(|error| format!("{context}: {error}")),
        _ = cancellation.cancelled() => Err("下载已取消".to_string()),
    }
}

async fn read_body_with_cancellation(
    response: reqwest::Response,
    cancellation: &CancellationToken,
    context: &str,
) -> Result<bytes::Bytes, String> {
    tokio::select! {
        biased;
        body = response.bytes() => body.map_err(|error| format!("{context}: {error}")),
        _ = cancellation.cancelled() => Err("下载已取消".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(package: Option<MirrorchyanPackage>) -> AvailableUpdateMetadata {
        AvailableUpdateMetadata {
            version_name: "v1.3.0".to_string(),
            release_note: "notes".to_string(),
            mirrorchyan_package: package,
        }
    }

    fn package() -> MirrorchyanPackage {
        MirrorchyanPackage {
            url: "https://example.com/mirror.zip".to_string(),
            expected_sha256: Some("abc".to_string()),
            file_size: Some(42),
        }
    }

    fn asset(name: &str, size: u64, digest: Option<&str>) -> GithubAsset {
        GithubAsset {
            name: name.to_string(),
            url: format!("https://example.com/{name}"),
            size,
            digest: digest.map(str::to_string),
        }
    }

    #[test]
    fn selects_configured_source_and_mirrorchyan_fallback() {
        let package = package();
        assert_eq!(
            select_source(UpdateSource::Mirrorchyan, Some("cdk"), Some(&package)),
            SourceChoice::Mirrorchyan
        );
        assert_eq!(
            select_source(UpdateSource::Mirrorchyan, None, Some(&package)),
            SourceChoice::Oem
        );
        assert_eq!(
            select_source(UpdateSource::Mirrorchyan, Some("cdk"), None),
            SourceChoice::Oem
        );
        assert_eq!(
            select_source(UpdateSource::Oem, Some("cdk"), Some(&package)),
            SourceChoice::Oem
        );
        assert_eq!(
            select_source(UpdateSource::Github, Some("cdk"), Some(&package)),
            SourceChoice::Github
        );
    }

    #[test]
    fn mirrorchyan_plan_keeps_cached_transfer_metadata() {
        let package = package();
        let plan = mirrorchyan_plan(&metadata(Some(package.clone())), &package);
        assert_eq!(plan.source, UpdateSource::Mirrorchyan);
        assert_eq!(plan.url, package.url);
        assert_eq!(plan.expected_sha256, package.expected_sha256);
        assert_eq!(plan.total_size, package.file_size);
        assert_eq!(plan.accept, None);
    }

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
            parse_oem_manifest("v1.3.0", manifest()).unwrap().source,
            UpdateSource::Oem
        );
        assert!(parse_oem_manifest("v1.4.0", manifest()).is_err());
    }

    #[test]
    fn github_prefers_exact_asset_then_largest_zip() {
        let exact = default_filename("v1.3.0");
        let digest = format!("sha256:{}", "A".repeat(64));
        let selected = select_github_asset(
            "v1.3.0",
            GithubRelease {
                assets: vec![
                    asset("larger.zip", 100, None),
                    asset(&exact, 10, Some(&digest)),
                ],
            },
        )
        .unwrap();
        assert_eq!(selected.filename, exact);
        assert_eq!(selected.expected_sha256, Some("a".repeat(64)));

        let fallback = select_github_asset(
            "v1.3.0",
            GithubRelease {
                assets: vec![asset("small.zip", 10, None), asset("large.ZIP", 20, None)],
            },
        )
        .unwrap();
        assert_eq!(fallback.filename, "large.ZIP");
    }

    #[test]
    fn github_invalid_or_missing_digest_skips_verification() {
        assert_eq!(parse_github_digest(None), None);
        assert_eq!(parse_github_digest(Some("md5:abc")), None);
        assert_eq!(parse_github_digest(Some("sha256:abc")), None);
        assert_eq!(
            parse_github_digest(Some(&format!("sha256:{}", "g".repeat(64)))),
            None
        );
        assert_eq!(
            parse_github_digest(Some(&format!("SHA256:{}", "A".repeat(64)))),
            Some("a".repeat(64))
        );
    }
}
