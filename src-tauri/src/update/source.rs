use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;

use crate::config::{OeaConfig, UpdateProxyMode, UpdateSource};

use super::transaction::DownloadSource;

const MIRROR_BASES: [&str; 2] = [
    "https://mirrorchyan.com/api/resources/OEA/latest",
    "https://mirrorchyan.net/api/resources/OEA/latest",
];
const GITHUB_RELEASES: &str =
    "https://api.github.com/repos/Logical-Byte/open-endfield-assistant/releases/tags";

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: String,
    pub release_notes: String,
    pub artifact_url: String,
    pub sha256: String,
    pub size: Option<u64>,
    pub source: DownloadSource,
}

#[derive(Deserialize)]
struct MirrorResponse {
    code: i64,
    msg: String,
    data: Option<MirrorData>,
}

#[derive(Deserialize)]
struct MirrorData {
    version_name: String,
    release_note: String,
    url: Option<String>,
    sha256: Option<String>,
    filesize: Option<u64>,
    update_type: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

pub fn build_client(config: &OeaConfig) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(format!("OEA/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60));
    match config.update_proxy_mode {
        UpdateProxyMode::None => builder = builder.no_proxy(),
        UpdateProxyMode::System => {}
        UpdateProxyMode::Custom if !config.update_proxy_url.trim().is_empty() => {
            builder = builder.proxy(
                reqwest::Proxy::all(config.update_proxy_url.trim()).context("代理地址无效")?,
            );
        }
        UpdateProxyMode::Custom => bail!("自定义代理地址为空"),
    }
    builder.build().context("创建更新 HTTP 客户端失败")
}

pub fn check(config: &OeaConfig, current_version: &str) -> Result<Option<AvailableUpdate>> {
    let client = build_client(config)?;
    let mirror = fetch_mirror(&client, &config.mirrorchyan_cdk)?;
    let current = Version::parse(current_version).context("当前版本号无效")?;
    let latest_text = mirror.version_name.trim_start_matches('v');
    let latest = Version::parse(latest_text).context("更新源版本号无效")?;
    if latest <= current {
        return Ok(None);
    }

    if config.update_source == UpdateSource::Mirrorchyan
        && !config.mirrorchyan_cdk.trim().is_empty()
    {
        if mirror
            .update_type
            .as_deref()
            .is_some_and(|kind| kind != "full")
        {
            bail!("Mirror酱未提供完整更新包");
        }
        return Ok(Some(AvailableUpdate {
            version: latest_text.to_string(),
            release_notes: mirror.release_note,
            artifact_url: mirror.url.context("Mirror酱未提供下载地址")?,
            sha256: normalize_sha256(mirror.sha256.as_deref())
                .context("Mirror酱未提供有效 SHA-256")?,
            size: mirror.filesize,
            source: DownloadSource::Mirrorchyan,
        }));
    }

    let release: GithubRelease = client
        .get(format!("{GITHUB_RELEASES}/v{latest_text}"))
        .send()
        .context("请求 GitHub Release 失败")?
        .error_for_status()
        .context("GitHub Release 返回错误")?
        .json()
        .context("解析 GitHub Release 失败")?;
    let exact = format!("OEA-windows-x86_64-v{latest_text}.zip");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == exact)
        .or_else(|| {
            release
                .assets
                .iter()
                .filter(|asset| asset.name.to_ascii_lowercase().ends_with(".zip"))
                .max_by_key(|asset| asset.size)
        })
        .context("GitHub Release 中没有完整 ZIP")?;
    Ok(Some(AvailableUpdate {
        version: latest_text.to_string(),
        release_notes: mirror.release_note,
        artifact_url: asset.browser_download_url.clone(),
        sha256: normalize_sha256(asset.digest.as_deref()).context("GitHub 资产缺少有效 SHA-256")?,
        size: Some(asset.size),
        source: DownloadSource::Github,
    }))
}

fn fetch_mirror(client: &Client, cdk: &str) -> Result<MirrorData> {
    let mut last_error = None;
    for base in MIRROR_BASES {
        let mut url = reqwest::Url::parse(base).expect("固定 MirrorChyan URL 应有效");
        url.query_pairs_mut()
            .append_pair("user_agent", "oea_client")
            .append_pair("channel", "stable")
            .append_pair("os", "windows")
            .append_pair("arch", "amd64");
        if !cdk.trim().is_empty() {
            url.query_pairs_mut().append_pair("cdk", cdk.trim());
        }
        match client
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
        {
            Ok(response) => match response.json::<MirrorResponse>() {
                Ok(payload) if payload.code == 0 => {
                    return payload.data.context("Mirror酱响应缺少数据");
                }
                Ok(payload) => {
                    last_error = Some(anyhow::anyhow!("{} ({})", payload.msg, payload.code))
                }
                Err(error) => last_error = Some(error.into()),
            },
            Err(error) => last_error = Some(error.into()),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Mirror酱检查失败")))
}

fn normalize_sha256(value: Option<&str>) -> Option<String> {
    let raw = value?;
    let value = raw
        .strip_prefix("sha256:")
        .unwrap_or(raw)
        .to_ascii_lowercase();
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(value)
}
