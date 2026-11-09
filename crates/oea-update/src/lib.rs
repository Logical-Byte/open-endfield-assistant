//! OEA 的自更新内核。宿主负责设置、界面、helper 模式分派与主进程退出。
mod check;
mod download;
mod error;
mod http;
pub mod install;
mod manager;
mod platform;
mod response;
pub mod source;
mod updater;

pub use error::{Error, Reason};
pub use manager::{UpdateInfo, UpdateOperationStatus, UpdateStatus};
pub use updater::{DownloadOutcome, DownloadProgress, ProgressSink, UpdateAvailability, Updater};

/// 检查更新使用的宿主设置快照。CDK 已解密，代理已解析为具体地址。
#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub cdk: Option<String>,
    pub proxy_url: Option<String>,
}
