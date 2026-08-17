use anyhow::{Context, Result, bail};
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, WAIT_OBJECT_0},
        Storage::FileSystem::{REPLACE_FILE_FLAGS, ReplaceFileW},
        System::Threading::{INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    },
    core::HSTRING,
};

use crate::update::{bootstrap::BootstrapInput, transaction::CANDIDATE_EXE};

pub fn wait_for_process(pid: u32) -> Result<()> {
    let handle = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
        Ok(handle) => handle,
        Err(error) if error.code() == ERROR_INVALID_PARAMETER.to_hresult() => return Ok(()),
        Err(error) => return Err(error).context("打开调用方进程失败"),
    };
    let event = unsafe { WaitForSingleObject(handle, INFINITE) };
    unsafe { CloseHandle(handle) }.context("关闭调用方进程句柄失败")?;
    if event != WAIT_OBJECT_0 {
        bail!("等待调用方进程失败: {event:?}");
    }
    Ok(())
}

pub fn replace_entrypoint(input: &BootstrapInput, source_version: &str) -> Result<()> {
    let entrypoint = HSTRING::from(
        input
            .portable_root()
            .join("OEA.exe")
            .to_string_lossy()
            .as_ref(),
    );
    let candidate = HSTRING::from(
        input
            .transaction_dir()
            .join(CANDIDATE_EXE)
            .to_string_lossy()
            .as_ref(),
    );
    let backup = HSTRING::from(
        input
            .portable_root()
            .join(format!("OEA-backup-v{source_version}.exe"))
            .to_string_lossy()
            .as_ref(),
    );
    unsafe {
        ReplaceFileW(
            &entrypoint,
            &candidate,
            &backup,
            REPLACE_FILE_FLAGS(0),
            None,
            None,
        )
    }
    .context("原子替换 OEA.exe 失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use crate::{
        update::{
            bootstrap::{BootstrapInput, run},
            transaction::CANDIDATE_EXE,
        },
        windows_ops::update::NativePlatform,
    };

    use super::replace_entrypoint;

    fn system_executable(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join(name)
    }

    #[test]
    fn atomic_replace_creates_root_entrypoint_and_backup() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(transaction.join("candidate")).unwrap();
        fs::write(root.join("OEA.exe"), b"old").unwrap();
        fs::write(transaction.join(CANDIDATE_EXE), b"new").unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();

        replace_entrypoint(&input, "0.1.0").unwrap();

        assert_eq!(fs::read(root.join("OEA.exe")).unwrap(), b"new");
        assert_eq!(
            fs::read(root.join("OEA-backup-v0.1.0.exe")).unwrap(),
            b"old"
        );
    }

    #[test]
    fn bootstrap_waits_for_real_process_then_replaces_and_launches() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(transaction.join("candidate")).unwrap();
        fs::copy(system_executable("where.exe"), root.join("OEA.exe")).unwrap();
        fs::copy(
            system_executable("where.exe"),
            transaction.join(CANDIDATE_EXE),
        )
        .unwrap();
        let mut child = Command::new("cmd.exe")
            .args(["/C", "ping -n 2 127.0.0.1 >NUL"])
            .spawn()
            .unwrap();
        let input = BootstrapInput::new(&root, &transaction).unwrap();

        run(&input, child.id(), "0.1.0", &NativePlatform).unwrap();
        child.wait().unwrap();

        assert!(root.join("OEA-backup-v0.1.0.exe").is_file());
    }
}
