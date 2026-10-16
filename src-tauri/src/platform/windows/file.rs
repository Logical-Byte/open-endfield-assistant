//! Windows 文件提交原语。

use std::{io, os::windows::ffi::OsStrExt, path::Path};

use ::windows::{
    Win32::Storage::FileSystem::ReplaceFileW,
    core::{PCWSTR, Result as WindowsResult},
};

pub(in crate::platform) fn replace(replacement: &Path, target: &Path) -> io::Result<()> {
    if !target.exists() {
        return std::fs::rename(replacement, target);
    }

    let replacement_wide = path_to_wide(replacement);
    let target_wide = path_to_wide(target);
    replace_file_w(
        PCWSTR::from_raw(target_wide.as_ptr()),
        PCWSTR::from_raw(replacement_wide.as_ptr()),
        PCWSTR::null(),
    )
    .map_err(|error| io::Error::other(error.to_string()))
}

fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn replace_file_w(target: PCWSTR, replacement: PCWSTR, backup: PCWSTR) -> WindowsResult<()> {
    unsafe { ReplaceFileW(target, replacement, backup, Default::default(), None, None) }
}
