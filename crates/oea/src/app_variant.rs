//! OEA 应用形态，与 Rust 的编译优化等级和版本发布事实无关。

use ts_rs::TS;

/// OEA dev 与 OEA release 的应用形态。
/// 两端共享枚举定义，各自通过构建工具确定当前值，不通过 IPC 同步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(export, export_to = "app/", repr(enum = name))]
pub enum AppVariant {
    Dev,
    Release,
}

/// Rust 编译期确定的应用形态，不受 debug/release profile 影响。
pub const CURRENT: AppVariant = if tauri::is_dev() {
    AppVariant::Dev
} else {
    AppVariant::Release
};
