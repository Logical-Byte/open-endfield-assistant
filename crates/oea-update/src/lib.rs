//! OEA 的自更新内核。宿主负责设置、界面、helper 参数分派与退出。
mod check;
mod download;
mod error;
mod http;
pub mod install;
mod manager;
mod platform;
mod response;
mod source;
mod updater;

pub use error::{Error, Reason};
pub use manager::{UpdateInfo, UpdateOperationStatus, UpdateStatus};
pub use updater::{DownloadOutcome, DownloadProgress, ProgressSink, UpdateAvailability, Updater};

/// 一次操作使用的宿主设置快照。CDK 已解密，代理已解析为具体地址。
#[derive(Debug, Clone)]
pub struct Options {
    pub source: UpdateSource,
    pub cdk: Option<String>,
    /// 系统代理解析失败只在请求实际使用代理配置时报告，Mirror酱下载仍可直连。
    pub proxy_url: Result<Option<String>, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSource {
    Mirrorchyan,
    Oem,
    Github,
}
