use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::transaction::{TRANSACTION_FILE, Transaction};

/// 已验证的 Bootstrap 路径输入。
#[derive(Debug, Clone)]
pub struct BootstrapInput {
    portable_root: PathBuf,
    transaction_dir: PathBuf,
}

impl BootstrapInput {
    pub fn new(portable_root: &Path, transaction_dir: &Path) -> Result<Self> {
        if !portable_root.is_absolute() || !transaction_dir.is_absolute() {
            bail!("Bootstrap 路径必须是绝对路径");
        }
        let updates_dir = portable_root.join("cache").join("updates");
        if transaction_dir.parent() != Some(updates_dir.as_path()) {
            bail!("事务目录不在便携根目录的更新缓存中");
        }
        if let (Ok(canonical_updates), Ok(canonical_transaction)) =
            (updates_dir.canonicalize(), transaction_dir.canonicalize())
        {
            if canonical_transaction.parent() != Some(canonical_updates.as_path()) {
                bail!("事务目录通过符号链接越过了便携根目录");
            }
        }
        Ok(Self {
            portable_root: portable_root.to_path_buf(),
            transaction_dir: transaction_dir.to_path_buf(),
        })
    }

    pub fn portable_root(&self) -> &Path {
        &self.portable_root
    }

    pub fn transaction_dir(&self) -> &Path {
        &self.transaction_dir
    }
}

/// Bootstrap 唯一的平台差异边界。
pub trait PlatformOps {
    fn wait_for_process(&self, pid: u32) -> Result<()>;
    fn replace_entrypoint(&self, input: &BootstrapInput, source_version: &str) -> Result<()>;
    fn launch_entrypoint(&self, input: &BootstrapInput) -> Result<()>;
}

/// 等待旧进程、替换根入口，再启动新入口。
pub fn run(
    input: &BootstrapInput,
    caller_pid: u32,
    source_version: &str,
    platform: &impl PlatformOps,
) -> Result<()> {
    platform.wait_for_process(caller_pid)?;
    platform.replace_entrypoint(input, source_version)?;
    if let Err(error) = platform.launch_entrypoint(input) {
        let backup = format!("OEA-backup-v{source_version}.exe");
        let message = format!("更新后的 OEA.exe 启动失败: {error:#}\n旧版本备份: {backup}\n");
        fs_err_write(
            &input.portable_root.join("update-error.txt"),
            message.as_bytes(),
        )?;
        return Err(error);
    }
    Ok(())
}

fn fs_err_write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).with_context(|| format!("写入 {} 失败", path.display()))
}

/// 在普通应用初始化前识别并执行 Bootstrap 模式。
pub fn try_run_from_args() -> Option<i32> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().and_then(|arg| arg.to_str()) != Some("--bootstrap-update") {
        return None;
    }
    let result = (|| -> Result<()> {
        if args.len() != 3 {
            bail!("--bootstrap-update 需要便携根目录和事务目录");
        }
        let input = BootstrapInput::new(Path::new(&args[1]), Path::new(&args[2]))?;
        let transaction = Transaction::load(&input.transaction_dir.join(TRANSACTION_FILE))?;
        if transaction.stage != super::transaction::TransactionStage::BootstrapReady {
            bail!("事务尚未准备好 Bootstrap");
        }
        let caller_pid = transaction.caller_pid.context("事务缺少调用方 PID")?;
        run(
            &input,
            caller_pid,
            &transaction.source_version,
            &crate::windows_ops::update::NativePlatform,
        )
    })();
    match result {
        Ok(()) => Some(0),
        Err(error) => {
            eprintln!("Bootstrap update failed: {error:#}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Mutex};

    use anyhow::{Result, bail};

    use super::{BootstrapInput, PlatformOps, run};

    struct FakeOps {
        calls: Mutex<Vec<&'static str>>,
        fail_at: Option<&'static str>,
    }

    impl PlatformOps for FakeOps {
        fn wait_for_process(&self, _pid: u32) -> Result<()> {
            self.calls.lock().unwrap().push("wait");
            if self.fail_at == Some("wait") {
                bail!("wait failed")
            } else {
                Ok(())
            }
        }

        fn replace_entrypoint(&self, _input: &BootstrapInput, _source_version: &str) -> Result<()> {
            self.calls.lock().unwrap().push("replace");
            if self.fail_at == Some("replace") {
                bail!("replace failed")
            } else {
                Ok(())
            }
        }

        fn launch_entrypoint(&self, _input: &BootstrapInput) -> Result<()> {
            self.calls.lock().unwrap().push("launch");
            if self.fail_at == Some("launch") {
                bail!("launch failed")
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn accepts_transaction_inside_portable_update_cache() {
        let input = BootstrapInput::new(
            Path::new("/portable"),
            Path::new("/portable/cache/updates/current"),
        )
        .unwrap();

        assert_eq!(input.portable_root(), Path::new("/portable"));
    }

    #[test]
    fn rejects_relative_or_out_of_root_paths() {
        assert!(BootstrapInput::new(Path::new("portable"), Path::new("updates/current")).is_err());
        assert!(
            BootstrapInput::new(Path::new("/portable"), Path::new("/outside/current")).is_err()
        );
    }

    #[test]
    fn bootstrap_waits_then_replaces_then_launches() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        std::fs::create_dir_all(&transaction).unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();
        let ops = FakeOps {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
        };

        run(&input, 123, "0.1.0", &ops).unwrap();

        assert_eq!(*ops.calls.lock().unwrap(), ["wait", "replace", "launch"]);
    }

    #[test]
    fn bootstrap_stops_after_replace_failure() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        std::fs::create_dir_all(&transaction).unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();
        let ops = FakeOps {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("replace"),
        };

        assert!(run(&input, 123, "0.1.0", &ops).is_err());
        assert_eq!(*ops.calls.lock().unwrap(), ["wait", "replace"]);
    }

    #[test]
    fn bootstrap_stops_after_wait_failure() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        std::fs::create_dir_all(&transaction).unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();
        let ops = FakeOps {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("wait"),
        };

        assert!(run(&input, 123, "0.1.0", &ops).is_err());
        assert_eq!(*ops.calls.lock().unwrap(), ["wait"]);
    }

    #[test]
    fn launch_failure_writes_manual_recovery_instructions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        std::fs::create_dir_all(&transaction).unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();
        let ops = FakeOps {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("launch"),
        };

        assert!(run(&input, 123, "0.1.0", &ops).is_err());

        let error = std::fs::read_to_string(root.join("update-error.txt")).unwrap();
        assert!(error.contains("OEA-backup-v0.1.0.exe"));
    }
}
