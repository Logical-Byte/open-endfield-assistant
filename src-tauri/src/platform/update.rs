//! 更新流程需要的跨进程文件原语。
//!
//! 更新模块只依赖这里提供的文件锁和替换接口。Windows 的句柄、锁标志和
//! `ReplaceFileW` 都留在 `platform::windows` 内，Unix 实现使用文件描述符上的
//! `flock`。调用方因此不需要知道当前平台的原生类型。

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

#[cfg(target_os = "macos")]
use std::process::{Child, Command};

#[cfg(target_os = "windows")]
use super::windows;

/// 更新提示模式。测试和无 UI 的 helper 可选择 `Silent`，不会创建或等待系统窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePromptMode {
    Interactive,
    Silent,
}

impl UpdatePromptMode {
    /// 测试构建或设置 `OEA_UPDATE_SILENT` 时关闭原生窗口。
    pub fn from_environment() -> Self {
        if cfg!(test)
            || std::env::var_os("OEA_UPDATE_SILENT").is_some()
            || std::env::var_os("OEA_UPDATE_TEST_MODE").is_some()
        {
            Self::Silent
        } else {
            Self::Interactive
        }
    }
}

/// 更新过程状态提示。
///
/// `new` 会尽力创建原生状态窗；若当前桌面环境不能创建窗口，会退化为日志，不能
/// 阻断文件事务。`show_success`/`show_error` 会先关闭进度窗，再显示最终结果。
pub struct UpdatePrompt {
    mode: UpdatePromptMode,
    #[cfg(target_os = "windows")]
    status_window: Option<windows::update::StatusWindow>,
    #[cfg(target_os = "macos")]
    status_process: Option<Child>,
}

impl UpdatePrompt {
    pub fn new(title: &str, content: &str, mode: UpdatePromptMode) -> Self {
        if mode == UpdatePromptMode::Silent {
            return Self::silent(mode);
        }

        #[cfg(target_os = "windows")]
        {
            match windows::update::StatusWindow::show(title, content) {
                Ok(status_window) => Self {
                    mode,
                    status_window: Some(status_window),
                },
                Err(error) => {
                    tracing::warn!("创建更新原生状态窗失败: {error}");
                    Self::silent(mode)
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            match spawn_macos_status(title, content) {
                Ok(status_process) => Self {
                    mode,
                    status_process: Some(status_process),
                },
                Err(error) => {
                    tracing::warn!("创建 macOS 更新状态窗失败: {error}");
                    Self::silent(mode)
                }
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = (title, content);
            tracing::info!("更新状态: {content}");
            Self::silent(mode)
        }
    }

    fn silent(mode: UpdatePromptMode) -> Self {
        Self {
            mode,
            #[cfg(target_os = "windows")]
            status_window: None,
            #[cfg(target_os = "macos")]
            status_process: None,
        }
    }

    pub fn finish(&mut self) {
        self.close_progress();
    }

    pub fn show_success(&mut self, title: &str, content: &str) {
        self.close_progress();
        if self.mode != UpdatePromptMode::Interactive {
            return;
        }
        if let Err(error) = show_final_message(title, content, FinalMessageKind::Success) {
            tracing::warn!("显示更新成功提示失败: {error}");
        }
    }

    pub fn show_error(&mut self, title: &str, content: &str) {
        self.close_progress();
        if self.mode != UpdatePromptMode::Interactive {
            return;
        }
        if let Err(error) = show_final_message(title, content, FinalMessageKind::Error) {
            tracing::warn!("显示更新失败提示失败: {error}");
        }
    }

    fn close_progress(&mut self) {
        #[cfg(target_os = "windows")]
        self.status_window.take();

        #[cfg(target_os = "macos")]
        {
            if let Some(mut process) = self.status_process.take() {
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }
}

impl Drop for UpdatePrompt {
    fn drop(&mut self) {
        self.close_progress();
    }
}

#[derive(Clone, Copy)]
enum FinalMessageKind {
    Success,
    Error,
}

fn show_final_message(title: &str, content: &str, kind: FinalMessageKind) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let icon = match kind {
            FinalMessageKind::Success => crate::platform::dialog::DialogIcon::Info,
            FinalMessageKind::Error => crate::platform::dialog::DialogIcon::Error,
        };
        windows::update::show_message(title, content, icon)
    }

    #[cfg(target_os = "macos")]
    {
        show_macos_message(title, content, kind)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (title, content, kind);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn apple_script_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn spawn_macos_status(title: &str, content: &str) -> io::Result<Child> {
    let script = format!(
        "repeat\ndisplay dialog \"{}\" with title \"{}\" buttons {{\"正在处理\"}} default button \"正在处理\"\nend repeat",
        apple_script_string(content),
        apple_script_string(title),
    );
    Command::new("osascript").args(["-e", &script]).spawn()
}

#[cfg(target_os = "macos")]
fn show_macos_message(title: &str, content: &str, kind: FinalMessageKind) -> io::Result<()> {
    let button = match kind {
        FinalMessageKind::Success | FinalMessageKind::Error => "确定",
    };
    let script = format!(
        "display dialog \"{}\" with title \"{}\" buttons {{\"{}\"}} default button \"{}\"",
        apple_script_string(content),
        apple_script_string(title),
        button,
        button,
    );
    Command::new("osascript").args(["-e", &script]).status()?;
    Ok(())
}

/// 一个持有底层文件句柄的跨进程排他锁。
///
/// 锁文件本身可以永久存在。真正表示“持锁”的是这个值所持有的打开句柄，
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        if lock_file(&file, true)? {
            Ok(Some(Self { file }))
        } else {
            Ok(None)
        }
    }

    /// 阻塞直到获得 `path` 对应的排他锁。
    pub fn acquire(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        lock_file(&file, false)?;
        Ok(Self { file })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = unlock_file(&self.file);
    }
}

/// 原子地用 `replacement` 替换 `target`。
///
/// Windows 使用 `ReplaceFileW`，因为普通 `rename` 不能替换仍然存在的 exe；
/// macOS/Unix 的同文件系统 `rename` 本身就是原子替换。成功后 source 不再存在。
pub fn replace_file(replacement: &Path, target: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::update::replace_file(replacement, target)
    }

    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(replacement, target)
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
