//! 自动更新：下载、进度、取消与安装状态。

pub mod commands;
pub mod install;

mod check;
mod download;
mod http;
mod manager;
mod response;
mod source;

pub use manager::UpdateManager;
