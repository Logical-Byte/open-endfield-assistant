//! 自动化任务的停止信号与中断错误。

use std::sync::{Arc, atomic::AtomicBool};

/// 由控制器写入、由自动化会话在每次操作前读取的停止令牌。
pub(crate) type StopToken = Arc<AtomicBool>;

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
