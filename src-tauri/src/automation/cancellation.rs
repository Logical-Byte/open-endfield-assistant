//! 自动化任务的停止信号与中断错误。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// 由控制器写入、由自动化会话在每次操作前读取的停止令牌。
pub(crate) type StopToken = Arc<AtomicBool>;

/// 创建一个尚未请求停止的自动化运行令牌。
pub(crate) fn new_stop_token() -> StopToken {
    Arc::new(AtomicBool::new(false))
}

/// 请求当前自动化运行停止。
pub(crate) fn request_stop(stop: &StopToken) {
    stop.store(true, Ordering::Relaxed);
}

/// 读取当前自动化运行是否已收到停止请求。
pub(crate) fn is_stop_requested(stop: &StopToken) -> bool {
    stop.load(Ordering::Relaxed)
}

/// 自动化任务被用户请求停止。
///
/// 停止不是任务出错：运行时可以通过错误类型区分二者，并将停止映射为正常终态。
#[derive(Debug)]
pub(crate) struct AutomationStopped;

impl std::fmt::Display for AutomationStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "自动化任务已被用户停止")
    }
}

impl std::error::Error for AutomationStopped {}
