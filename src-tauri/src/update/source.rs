//! 更新源元数据查询。
//!
//! 这里只下载很小的版本、URL 与校验值元数据。真正的 ZIP 下载由 `download` 模块负责。
//! MirrorChyan 的端点、参数约定以及 GitHub/MirrorChyan 产品选择改编自 PR #6；
//! 与 PR #6 不同，网络访问和来源选择现在完全由 Rust 后端拥有。

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
const GITHUB_LATEST_RELEASE: &str =
    "https://api.github.com/repos/Logical-Byte/open-endfield-assistant/releases/latest";

/// 已通过来源规则验证、可以交给下载工作流的完整包描述。
#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    /// 不带 `v` 前缀的语义化版本号。
    pub version: String,
    /// 给前端展示的 Markdown 更新日志。
    pub release_notes: String,
    /// 完整 ZIP 的下载地址。
    pub artifact_url: String,
    /// 规范化为 64 位小写十六进制的 SHA-256。
    pub sha256: String,
    /// 来源已知文件大小时为 `Some`。
    pub size: Option<u64>,
    /// 实际提供该 ZIP 的来源。
    pub source: DownloadSource,
}

/// MirrorChyan API 的外层业务响应。
#[derive(Deserialize)]
struct MirrorResponse {
    code: i64,
    msg: String,
    data: Option<MirrorData>,
}

/// MirrorChyan 成功响应中的版本与完整包字段。
#[derive(Deserialize)]
struct MirrorData {
    version_name: String,
    release_note: String,
    url: Option<String>,
    sha256: Option<String>,
    filesize: Option<u64>,
    update_type: Option<String>,
}

/// GitHub `/releases/latest` 响应中本工作流需要的字段。
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

/// GitHub Release 附件的最小表示。
#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

/// 按用户代理设置构造阻塞式 HTTP 客户端。
///
/// 更新命令运行在 Tauri 的 blocking worker 中，因此这里使用 `reqwest` blocking API。
/// 连接超时限制握手等待；较长总超时允许大型完整包在慢速网络下载。
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

/// 根据配置选择一个来源并检查是否存在更高版本。
///
/// PR #6 的产品规则是：选择 MirrorChyan 且填写 CDK 时使用镜像，否则使用 GitHub。
/// 返回 `None` 表示当前版本已是最新，而不是请求失败。
pub fn check(config: &OeaConfig, current_version: &str) -> Result<Option<AvailableUpdate>> {
    let client = build_client(config)?;
    if config.update_source == UpdateSource::Mirrorchyan
        && !config.mirrorchyan_cdk.trim().is_empty()
    {
        check_mirror(&client, current_version, &config.mirrorchyan_cdk)
    } else {
        check_github(&client, current_version)
    }
}

/// 查询 MirrorChyan，并拒绝增量包或缺少 SHA-256 的响应。
fn check_mirror(
    client: &Client,
    current_version: &str,
    cdk: &str,
) -> Result<Option<AvailableUpdate>> {
    let mirror = fetch_mirror(client, cdk)?;
    let current = Version::parse(current_version).context("当前版本号无效")?;
    let latest_text = mirror.version_name.trim_start_matches('v');
    let latest = Version::parse(latest_text).context("更新源版本号无效")?;
    if latest <= current {
        return Ok(None);
    }

    if mirror.update_type.as_deref() != Some("full") {
        bail!("Mirror酱未提供完整更新包");
    }
    Ok(Some(AvailableUpdate {
        version: latest_text.to_string(),
        release_notes: mirror.release_note,
        artifact_url: mirror.url.context("Mirror酱未提供下载地址")?,
        sha256: normalize_sha256(mirror.sha256.as_deref()).context("Mirror酱未提供有效 SHA-256")?,
        size: mirror.filesize,
        source: DownloadSource::Mirrorchyan,
    }))
}

/// 独立查询 GitHub 最新 Release，并只接受精确命名的 Windows x86_64 完整包。
fn check_github(client: &Client, current_version: &str) -> Result<Option<AvailableUpdate>> {
    let release: GithubRelease = client
        .get(GITHUB_LATEST_RELEASE)
        .send()
        .context("请求 GitHub 最新 Release 失败")?
        .error_for_status()
        .context("GitHub 最新 Release 返回错误")?
        .json()
        .context("解析 GitHub Release 失败")?;
    let current = Version::parse(current_version).context("当前版本号无效")?;
    let latest_text = release.tag_name.trim_start_matches('v');
    let latest = Version::parse(latest_text).context("GitHub Release 版本号无效")?;
    if latest <= current {
        return Ok(None);
    }

    let asset = select_github_asset(&release.assets, latest_text)?;
    let sha256 = match normalize_sha256(asset.digest.as_deref()) {
        Some(digest) => digest,
        None => fetch_checksum(client, &release.assets, &asset.name)?
            .context("GitHub Release 未提供此完整包的有效 SHA-256")?,
    };
    Ok(Some(AvailableUpdate {
        version: latest_text.to_string(),
        release_notes: release.body.unwrap_or_default(),
        artifact_url: asset.browser_download_url.clone(),
        sha256,
        size: Some(asset.size),
        source: DownloadSource::Github,
    }))
}

/// 在 Release 附件中选择唯一允许的完整包文件名，不回退到任意 ZIP。
fn select_github_asset<'a>(assets: &'a [GithubAsset], version: &str) -> Result<&'a GithubAsset> {
    let exact = format!("OEA-windows-x86_64-v{version}.zip");
    assets
        .iter()
        .find(|asset| asset.name == exact)
        .with_context(|| format!("GitHub Release 缺少完整包 {exact}"))
}

/// 当 GitHub asset digest 缺失时，从 checksum 附件查找该 ZIP 的 SHA-256。
fn fetch_checksum(
    client: &Client,
    assets: &[GithubAsset],
    artifact_name: &str,
) -> Result<Option<String>> {
    for checksum_asset in assets.iter().filter(|asset| {
        let name = asset.name.to_ascii_lowercase();
        name.contains("sha256") || name.contains("checksum")
    }) {
        let contents = client
            .get(&checksum_asset.browser_download_url)
            .send()
            .with_context(|| format!("下载 GitHub 校验文件 {} 失败", checksum_asset.name))?
            .error_for_status()
            .with_context(|| format!("GitHub 校验文件 {} 返回错误", checksum_asset.name))?
            .text()
            .with_context(|| format!("读取 GitHub 校验文件 {} 失败", checksum_asset.name))?;
        if let Some(checksum) = checksum_for(&contents, artifact_name) {
            return Ok(Some(checksum));
        }
    }
    Ok(None)
}

/// 解析常见 checksum 文件格式，并要求文件名与所选 ZIP 完全相同。
fn checksum_for(contents: &str, artifact_name: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        if let Some((digest, name)) = line.split_once(char::is_whitespace) {
            let name = name.trim_start().trim_start_matches('*');
            if name == artifact_name {
                return normalize_sha256(Some(digest));
            }
        }
        let prefix = format!("SHA256 ({artifact_name}) = ");
        line.strip_prefix(&prefix)
            .and_then(|digest| normalize_sha256(Some(digest)))
    })
}

/// 依次请求 MirrorChyan 主、备域名，并返回第一个业务成功响应。
///
/// 查询参数名称沿用 PR #6 的 MirrorChyan 客户端实现。
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

/// 接受 GitHub 的 `sha256:<hex>` 或纯 `<hex>`，并规范化为小写。
fn normalize_sha256(value: Option<&str>) -> Option<String> {
    let raw = value?;
    let value = raw
        .strip_prefix("sha256:")
        .unwrap_or(raw)
        .to_ascii_lowercase();
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{GithubAsset, checksum_for, select_github_asset};

    fn asset(name: &str) -> GithubAsset {
        GithubAsset {
            name: name.into(),
            size: 42,
            browser_download_url: format!("https://example.test/{name}"),
            digest: None,
        }
    }

    #[test]
    fn github_requires_the_exact_windows_x86_64_full_package() {
        let assets = vec![
            asset("OEA-windows-aarch64-v0.2.0.zip"),
            asset("unrelated-large.zip"),
        ];

        assert!(select_github_asset(&assets, "0.2.0").is_err());
    }

    #[test]
    fn checksum_file_must_name_the_selected_asset_exactly() {
        let expected = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let contents = format!(
            "{expected}  OEA-windows-x86_64-v0.2.0.zip\n{}  other.zip\n",
            "f".repeat(64)
        );

        assert_eq!(
            checksum_for(&contents, "OEA-windows-x86_64-v0.2.0.zip"),
            Some(expected.into())
        );
        assert_eq!(checksum_for(&contents, "missing.zip"), None);
    }
}
