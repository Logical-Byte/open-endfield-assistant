//! 跨平台文件提交原语。

use std::{io, path::Path};

#[cfg(not(target_os = "windows"))]
use std::fs;

#[cfg(target_os = "windows")]
use super::windows;

/// 原子地用 `replacement` 替换 `target`。
///
/// 两个路径必须位于同一文件系统。成功后 `replacement` 不再存在。
pub fn replace(replacement: &Path, target: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::file::replace(replacement, target)
    }

    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(replacement, target)
    }
}
