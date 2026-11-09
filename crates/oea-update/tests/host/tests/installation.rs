mod support;
use oea_update::install::{HelperLaunch, HelperLocale, InstallTarget};
use std::{ffi::OsStr, fs, path::Path, process::Command};
use support::{APP, Installation};

fn commit(installation: &Installation, package: &Path) {
    let mut old = installation.install(package, "normal");
    old.expect("installed");
    let mut helper = installation.helper();
    helper.send("run");
    helper.expect("logging");
    helper.expect("progress:EnUs");
    old.send("exit");
    old.wait(true);
    helper.expect("success:EnUs");
    helper.expect("prompt-dropped");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.closed();
}

#[test]
fn full_zip_updates_the_running_host_and_preserves_settings() {
    let installation = Installation::new();
    commit(&installation, &installation.full_package());
    assert!(installation.start("Completed", 2));
    installation.assert_updated();
    assert!(!installation.start("NoTransaction", 2));
}

#[test]
fn incremental_zip_applies_changes_with_or_without_a_new_executable() {
    for executable in [true, false] {
        let installation = Installation::new();
        let modified_executable = if executable {
            format!("\"{APP}\",")
        } else {
            String::new()
        };
        let changes = format!(
            r#"{{"modified":[{modified_executable}"resources/changed.txt"],"added":["resources/new.txt","resources/added-dir/item.txt"],"deleted":["resources/old.txt","resources/old-dir/item.txt"],"added_dir":["resources/added-dir"],"deleted_dir":["resources/old-dir"]}}"#
        );
        let package = installation.package(
            executable,
            &[
                ("changes.json", changes.as_bytes()),
                ("resources/new.txt", b"new resource"),
                ("resources/changed.txt", b"after"),
                ("resources/added-dir/item.txt", b"added"),
            ],
        );
        commit(&installation, &package);
        installation.start("Completed", if executable { 2 } else { 1 });
        installation.assert_resources_updated();
        let expected = if executable {
            env!("CARGO_BIN_EXE_update-host-v2")
        } else {
            env!("CARGO_BIN_EXE_update-host-v1")
        };
        assert_eq!(
            fs::read(installation.root.path().join(APP)).unwrap(),
            fs::read(expected).unwrap()
        );
        let resources = installation.root.path().join("resources");
        assert_eq!(
            fs::read_to_string(resources.join("keep.txt")).unwrap(),
            "unchanged"
        );
        assert_eq!(
            fs::read_to_string(resources.join("changed.txt")).unwrap(),
            "after"
        );
        assert_eq!(
            fs::read_to_string(resources.join("added-dir/item.txt")).unwrap(),
            "added"
        );
        assert!(!resources.join("old-dir").exists());
    }
}

#[test]
fn failed_installation_preserves_old_payload_and_cancels_prepared_material() {
    for mode in [
        "copy-failure",
        "spawn-failure",
        "corrupt",
        "incomplete",
        "unsafe-incremental",
        "escaping-zip",
    ] {
        let installation = Installation::new();
        let package = match mode {
            "incomplete" => installation.package(false, &[("resources/new.txt", b"new resource")]),
            // 被删除目标是实际存在的宿主设置文件。若路径校验失效，操作能够执行，
            // 不会因源文件缺失而以其他原因失败。
            "unsafe-incremental" => installation.package(
                true,
                &[("changes.json", br#"{"deleted":["../../../settings.json"]}"#)],
            ),
            "escaping-zip" => installation.package(true, &[("../escape", b"invalid")]),
            _ => installation.full_package(),
        };
        if mode == "corrupt" {
            fs::write(&package, "not a ZIP").unwrap();
        }
        let mut old = installation.install(&package, mode);
        old.expect(if matches!(mode, "copy-failure" | "spawn-failure") {
            "error:HelperStart"
        } else {
            "error:Preparation"
        });
        old.wait(false);
        installation.assert_old();
        assert!(!package.exists(), "{mode}");
        assert!(
            !installation
                .root
                .path()
                .join("cache/update/transaction.json")
                .exists(),
            "{mode}"
        );
        assert!(
            !installation
                .root
                .path()
                .join("cache/update/candidate")
                .exists(),
            "{mode}"
        );
        assert!(
            !installation
                .root
                .path()
                .join("cache/update/escape")
                .exists()
        );
        installation.start("NoTransaction", 1);
    }
}

#[test]
fn cancelling_package_selection_preserves_zip_and_releases_installation() {
    let installation = Installation::new();
    let package = installation.full_package();
    let mut old = installation.install(&package, "cancel");
    old.expect("busy");
    old.expect("cancelled");
    old.wait(true);
    installation.assert_old();
    assert!(package.exists());
    installation.start("NoTransaction", 1);
}

#[test]
fn startup_defers_resources_until_helper_commits_executable() {
    let installation = Installation::new();
    let mut old = installation.install(&installation.full_package(), "normal");
    old.expect("installed");
    let mut helper = installation.helper();
    assert!(!installation.start("WaitingForHelper", 1));
    installation.assert_old();
    helper.send("run");
    helper.expect("logging");
    helper.expect("progress:EnUs");
    old.send("exit");
    old.wait(true);
    helper.expect("success:EnUs");
    helper.expect("prompt-dropped");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.closed();
    installation.start("Completed", 2);
    installation.assert_updated();
}

#[cfg(windows)]
#[test]
fn locked_old_executable_is_replaced_after_host_exits() {
    let installation = Installation::new();
    let mut old = installation.install(&installation.full_package(), "hold-executable");
    old.expect("installed");
    let mut helper = installation.helper();
    helper.send("run");
    helper.expect("logging");
    helper.expect("progress:EnUs");
    helper.assert_waiting();
    assert!(old.is_running());
    installation.assert_old();
    old.send("exit");
    old.wait(true);
    helper.expect("success:EnUs");
    helper.expect("prompt-dropped");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.closed();
    installation.start("Completed", 2);
    installation.assert_updated();
}

#[test]
fn startup_recovers_each_persisted_resources_interruption() {
    for phase in 0..3 {
        let installation = Installation::new();
        commit(&installation, &installation.full_package());
        let root = installation.root.path();
        // 磁盘事务协议允许在这两个资源移动操作后或 marker 删除前恢复。
        // 精确构造中断现场，避免依赖杀进程命中短暂的文件操作间隙。
        if phase >= 1 {
            fs::create_dir_all(root.join("cache/update/discard")).unwrap();
            fs::rename(
                root.join("resources"),
                root.join("cache/update/discard/resources"),
            )
            .unwrap();
        }
        if phase >= 2 {
            fs::rename(
                root.join("cache/update/candidate/resources"),
                root.join("resources"),
            )
            .unwrap();
            fs::remove_dir(root.join("cache/update/candidate")).unwrap();
        }
        assert_eq!(installation.start("Completed", 2), phase != 2);
        installation.assert_updated();
        assert!(!installation.start("NoTransaction", 2));
    }
}

/// 从真实 ZIP 构建事务并停在 helper 提交前，不直接构造内部 candidate 或 marker。
fn prepare(installation: &Installation) {
    let mut old = installation.install(&installation.full_package(), "normal");
    old.expect("installed");
    let helper = installation.helper();
    old.send("exit");
    old.wait(true);
    drop(helper);
}

#[test]
fn startup_rejects_malformed_or_unsupported_transaction_without_moving_payload() {
    for marker in ["{", r#"{"schema_version":2}"#] {
        let installation = Installation::new();
        prepare(&installation);
        let marker_path = installation
            .root
            .path()
            .join("cache/update/transaction.json");
        fs::write(&marker_path, marker).unwrap();
        let mut process = installation.start_process(1);
        process.expect("startup-error");
        process.wait(false);
        installation.assert_old();
        assert_eq!(fs::read_to_string(marker_path).unwrap(), marker);
    }
}

fn helper_command(installation: &Installation) -> Command {
    let target = InstallTarget::new(installation.root.path(), APP).unwrap();
    let launch = HelperLaunch::new(
        installation.helper_path(),
        ["helper-entry".into()],
        &target,
        HelperLocale::EnUs,
    );
    let mut command = Command::new(&launch.source_executable);
    command.args(launch.args);
    command
}

#[test]
fn helper_rejects_invalid_arguments_or_identity_before_initializing_logs() {
    for invalid in ["missing", "relative-root", "executable-path", "identity"] {
        let installation = Installation::new();
        prepare(&installation);
        let target = InstallTarget::new(installation.root.path(), APP).unwrap();
        let mut args = HelperLaunch::new(
            "unused",
            ["helper-entry".into()],
            &target,
            HelperLocale::EnUs,
        )
        .args;
        match invalid {
            "missing" => {
                args.drain(1..5);
            }
            "relative-root" => args[2] = "relative".into(),
            "executable-path" => args[4] = "../app".into(),
            _ => {}
        }
        let mut command = Command::new(if invalid == "identity" {
            installation.root.path().join(APP)
        } else {
            installation.helper_path()
        });
        command.args(args);
        let mut helper = installation.spawn(command, "helper:v1");
        helper.send("run");
        helper.expect("error:EnUs");
        helper.expect("host-dropped");
        helper.wait(false);
        assert!(!installation.root.path().join("logs").exists());
        installation.assert_old();
    }
}

#[test]
fn helper_uses_host_default_when_locale_is_missing_invalid_or_has_no_value() {
    for language in ["missing", "invalid", "missing-value", "before-target"] {
        let installation = Installation::new();
        prepare(&installation);
        let target = InstallTarget::new(installation.root.path(), APP).unwrap();
        let mut args = HelperLaunch::new(
            "unused",
            ["helper-entry".into()],
            &target,
            HelperLocale::EnUs,
        )
        .args;
        match language {
            "missing" => args.truncate(args.len() - 2),
            "invalid" => *args.last_mut().unwrap() = "invalid".into(),
            "missing-value" => {
                args.pop();
            }
            _ => {
                args.truncate(args.len() - 2);
                args.insert(1, "--oea-update-ui-locale".into());
            }
        }
        let mut command = Command::new(installation.helper_path());
        command.args(args);
        let mut helper = installation.spawn(command, "helper:v1");
        helper.send("run");
        helper.expect("logging");
        helper.expect("progress:ZhCn");
        helper.expect("success:ZhCn");
        helper.expect("prompt-dropped");
        helper.expect("log-dropped");
        helper.expect("host-dropped");
        helper.wait(true);
        installation.start("Completed", 2);
        installation.assert_updated();
    }
}

#[test]
fn helper_reports_corrupt_transaction_and_releases_logging_before_exit() {
    let installation = Installation::new();
    prepare(&installation);
    fs::write(
        installation
            .root
            .path()
            .join("cache/update/transaction.json"),
        "{",
    )
    .unwrap();
    let mut helper = installation.spawn(helper_command(&installation), "helper:v1");
    helper.send("run");
    helper.expect("logging");
    helper.expect("error:EnUs");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.wait(false);
    installation.assert_old();
}

#[test]
fn startup_without_update_does_not_create_transaction_lock() {
    let installation = Installation::new();
    assert!(!installation.start("NoTransaction", 1));
    assert!(
        !installation
            .root
            .path()
            .join("cache/update/transaction.lock")
            .exists()
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn full_zip_installation_preserves_native_non_utf8_root_path() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let installation = Installation::with_prefix(&OsString::from_vec(b"update-\xff".to_vec()));
    commit(&installation, &installation.full_package());
    installation.start("Completed", 2);
    installation.assert_updated();
}

#[test]
fn full_zip_installation_handles_unicode_and_spaces() {
    let installation = Installation::with_prefix(OsStr::new("更新 空间 "));
    commit(&installation, &installation.full_package());
    installation.start("Completed", 2);
    installation.assert_updated();
}

#[test]
fn repeated_helper_does_not_commit_again_or_show_progress() {
    let installation = Installation::new();
    commit(&installation, &installation.full_package());
    let mut helper = installation.spawn(helper_command(&installation), "helper:v1");
    helper.send("run");
    helper.expect("logging");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.wait(true);
    installation.start("Completed", 2);
    installation.assert_updated();
    // 启动恢复会清理 helper 副本。只重建程序，不重建已经结束的事务。
    fs::copy(
        env!("CARGO_BIN_EXE_update-host-v1"),
        installation.helper_path(),
    )
    .unwrap();
    let mut helper = installation.spawn(helper_command(&installation), "helper:v1");
    helper.send("run");
    helper.expect("logging");
    helper.expect("log-dropped");
    helper.expect("host-dropped");
    helper.wait(true);
    installation.assert_updated();
}
