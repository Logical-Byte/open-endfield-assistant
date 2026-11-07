//! GitHub Release 资产的请求与选择。
use super::{
    AvailableUpdateMetadata, DownloadPlan, default_filename, read_body_with_cancellation,
    send_with_cancellation,
};
use crate::{Error, Reason, http, response};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

const GITHUB_RELEASE_TAG_URL: &str =
    "https://api.github.com/repos/Logical-Byte/open-endfield-assistant/releases/tags/";

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

pub(super) async fn resolve(
    metadata: &AvailableUpdateMetadata,
    proxy_url: Option<&str>,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, Error> {
    let client = http::build_client(proxy_url, user_agent)?;
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
        return Err(Error::failed(
            Reason::PackageUnavailable {
                version: metadata.version_name.clone(),
            },
            anyhow::anyhow!("GitHub 上未找到版本 {} 的 Release", metadata.version_name),
        ));
    }
    if !response.status().is_success() {
        return Err(Error::failed(
            Reason::Network,
            anyhow::anyhow!(
                "GitHub API 错误（HTTP {}），GitHub 下载暂不可用",
                response.status()
            ),
        ));
    }
    let body =
        read_body_with_cancellation(response, cancellation, "GitHub API 响应读取失败").await?;
    let release: GithubRelease = serde_json::from_slice(&body).map_err(|error| {
        Error::failed(
            Reason::InvalidMetadata,
            anyhow::Error::new(error).context("GitHub API 响应不是有效 JSON"),
        )
    })?;
    select_github_asset(&metadata.version_name, release)
}

fn select_github_asset(version_name: &str, release: GithubRelease) -> Result<DownloadPlan, Error> {
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
        .ok_or_else(|| {
            Error::failed(
                Reason::PackageUnavailable {
                    version: version_name.to_string(),
                },
                anyhow::anyhow!("GitHub Release 中未找到 OEA-windows-x86_64 的 zip 资产"),
            )
        })?;

    let expected_sha256 = parse_github_digest(asset.digest.as_deref());
    Ok(DownloadPlan {
        url: asset.url,
        filename: response::sanitize_filename(&asset.name)
            .unwrap_or_else(|| default_filename(version_name)),
        expected_sha256,
        total_size: Some(asset.size),
        source_label: "GitHub",
        proxy_url: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    fn asset(name: &str, size: u64, digest: Option<&str>) -> GithubAsset {
        GithubAsset {
            name: name.to_string(),
            url: format!("https://example.com/{name}"),
            size,
            digest: digest.map(str::to_string),
        }
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
