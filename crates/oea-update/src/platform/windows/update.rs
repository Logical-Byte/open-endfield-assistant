//! Windows 更新文件锁原语。
use ::windows::Win32::{
    Foundation::{ERROR_LOCK_VIOLATION, HANDLE},
    Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
    },
    System::IO::OVERLAPPED,
};
use std::{fs::File, io, os::windows::io::AsRawHandle};

/// 获取文件上的 Windows 排他锁。
pub(in crate::platform) fn lock_file(file: &File, nonblocking: bool) -> io::Result<bool> {
    let mut flags = LOCKFILE_EXCLUSIVE_LOCK;
    if nonblocking {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    let mut overlapped = OVERLAPPED::default();
    let result = unsafe {
        LockFileEx(
            HANDLE(file.as_raw_handle()),
            flags,
            None,
            1,
            0,
            &mut overlapped,
        )
    };

    match result {
        Ok(()) => Ok(true),
        Err(error) if nonblocking && (error.code().0 as u32 & 0xffff) == ERROR_LOCK_VIOLATION.0 => {
            Ok(false)
        }
        Err(error) => Err(io::Error::other(error.to_string())),
    }
}

/// 释放文件上的 Windows 排他锁。
pub(in crate::platform) fn unlock_file(file: &File) -> io::Result<()> {
    let mut overlapped = OVERLAPPED::default();
    unsafe { UnlockFileEx(HANDLE(file.as_raw_handle()), None, 1, 0, &mut overlapped) }
        .map_err(|error| io::Error::other(error.to_string()))
}
