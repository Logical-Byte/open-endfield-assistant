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
#[cfg(all(feature = "legacy-tauri", not(feature = "desktop")))]
pub const CURRENT: AppVariant = if tauri::is_dev() {
    AppVariant::Dev
} else {
    AppVariant::Release
};

/// 原生入口的开发构建读取工作区资源，发行构建读取可执行文件旁的资源。
#[cfg(any(feature = "desktop", not(feature = "legacy-tauri")))]
pub const CURRENT: AppVariant = if cfg!(debug_assertions) {
    AppVariant::Dev
} else {
    AppVariant::Release
};
