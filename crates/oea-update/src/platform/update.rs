//! 更新事务的跨进程文件锁。
#[cfg(target_os = "windows")]
use super::windows;
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

/// 一个持有底层文件句柄的跨进程排他锁。
///
/// 锁文件本身可以永久存在。真正表示 “持锁” 的是这个值所持有的打开句柄，
/// 进程退出或值析构后，操作系统会释放排他锁，所以不会产生 stale lock。
pub struct FileLock {
    file: File,
}

impl FileLock {
    /// 非阻塞地尝试获得 `path` 对应的排他锁。
    ///
    /// 返回 `Ok(None)` 只表示另一个进程当前持有锁；其他打开或系统错误会返回
    /// `Err`。锁文件的父目录会在打开前创建。
    pub fn try_acquire(path: &Path) -> io::Result<Option<Self>> {
        let file = open_lock_file(path)?;

        if lock_file(&file, true)? {
            Ok(Some(Self { file }))
        } else {
            Ok(None)
        }
    }

    /// 阻塞直到获得 `path` 对应的排他锁。
    pub fn acquire(path: &Path) -> io::Result<Self> {
        let file = open_lock_file(path)?;
        lock_file(&file, false)?;
        Ok(Self { file })
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Err(error) = unlock_file(&self.file) {
            tracing::warn!(error = %error, "释放更新事务文件锁失败");
        }
    }
}

#[cfg(target_os = "windows")]
fn lock_file(file: &File, nonblocking: bool) -> io::Result<bool> {
    windows::update::lock_file(file, nonblocking)
}

#[cfg(target_os = "windows")]
fn unlock_file(file: &File) -> io::Result<()> {
    windows::update::unlock_file(file)
}

#[cfg(unix)]
fn lock_file(file: &File, nonblocking: bool) -> io::Result<bool> {
    use std::os::fd::AsRawFd;

    let mut operation = libc::LOCK_EX;
    if nonblocking {
        operation |= libc::LOCK_NB;
    }
    let result = unsafe { libc::flock(file.as_raw_fd(), operation) };
    if result == 0 {
        return Ok(true);
    }

    let error = io::Error::last_os_error();
    if nonblocking && error.raw_os_error() == Some(libc::EAGAIN) {
        Ok(false)
    } else {
        Err(error)
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;

    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn lock_file(_file: &File, _nonblocking: bool) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "update file locking is unsupported on this platform",
    ))
}

#[cfg(not(any(unix, target_os = "windows")))]
fn unlock_file(_file: &File) -> io::Result<()> {
    Ok(())
}
