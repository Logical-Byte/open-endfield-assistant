//! 从宿主明确选择的下载源生成下载计划，不处理设置或下载源回退。
mod github;
mod mirrorchyan;
mod oem;

use crate::{Error, Reason};
use tokio_util::sync::CancellationToken;

/// 宿主已经选定的下载源。Mirror酱要求 CDK 并固定直连，其他源只接收可选代理。
#[derive(Clone)]
pub enum Source {
    Mirrorchyan { cdk: String },
    Oem { proxy_url: Option<String> },
    Github { proxy_url: Option<String> },
}

impl Source {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Mirrorchyan { .. } => "Mirror酱",
            Self::Oem { .. } => "OEM",
            Self::Github { .. } => "GitHub",
        }
    }
}

/// 检查更新取得的版本与下载元数据，仅在后端使用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdateMetadata {
    pub version_name: String,
    pub release_note: String,
    pub mirrorchyan_package: Option<MirrorchyanPackage>,
}

/// 检查请求取得的 Mirror酱安装包信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorchyanPackage {
    pub url: String,
    pub expected_sha256: Option<String>,
    pub file_size: Option<u64>,
}

/// 一次下载使用的包信息与已经确定的 HTTP 参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    pub url: String,
    pub filename: String,
    pub expected_sha256: Option<String>,
    pub total_size: Option<u64>,
    pub(crate) source_label: &'static str,
    pub(crate) proxy_url: Option<String>,
    pub(crate) accept: Option<&'static str>,
}

/// 解析指定源的下载计划，不隐式切换源。
///
/// Mirror酱复用检查更新时取得的安装包地址，要求宿主提供当前 CDK。
/// 缺少安装包时返回错误，由宿主在调用前决定是否选择其他源。
pub async fn resolve_download_plan(
    metadata: &AvailableUpdateMetadata,
    source: Source,
    user_agent: &str,
    cancellation: &CancellationToken,
) -> Result<DownloadPlan, Error> {
    match source {
        Source::Mirrorchyan { cdk } => mirrorchyan::resolve(metadata, &cdk),
        Source::Oem { proxy_url } => {
            let mut plan =
                oem::resolve(metadata, proxy_url.as_deref(), user_agent, cancellation).await?;
            plan.proxy_url = proxy_url;
            Ok(plan)
        }
        Source::Github { proxy_url } => {
            let mut plan =
                github::resolve(metadata, proxy_url.as_deref(), user_agent, cancellation).await?;
            plan.proxy_url = proxy_url;
            Ok(plan)
        }
    }
}

fn default_filename(version_name: &str) -> String {
    format!("OEA-windows-x86_64-{version_name}.zip")
}

async fn send_with_cancellation(
    request: reqwest::RequestBuilder,
    cancellation: &CancellationToken,
    context: &str,
) -> Result<reqwest::Response, Error> {
    tokio::select! {
        biased;
        response = request.send() => response.map_err(|error| Error::failed(Reason::Network, anyhow::Error::new(error).context(context.to_owned()))),
        _ = cancellation.cancelled() => Err(Error::new(Reason::Cancelled)),
    }
}

async fn read_body_with_cancellation(
    response: reqwest::Response,
    cancellation: &CancellationToken,
    context: &str,
) -> Result<bytes::Bytes, Error> {
    tokio::select! {
        biased;
        body = response.bytes() => body.map_err(|error| Error::failed(Reason::Network, anyhow::Error::new(error).context(context.to_owned()))),
        _ = cancellation.cancelled() => Err(Error::new(Reason::Cancelled)),
    }
}
