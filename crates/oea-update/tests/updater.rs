use std::{cell::RefCell, fs, io::Write, path::Path};

use oea_update::{
    Error, Reason, UpdateOperationStatus, Updater,
    install::{
        HelperLaunch, InstallStage, InstallTarget, StartupUpdateResult,
        complete_startup_transaction, validate_helper_request,
    },
};

fn full_package(root: &Path) -> std::path::PathBuf {
    let downloads = root.join("cache/downloads");
    fs::create_dir_all(&downloads).unwrap();
    let package = downloads.join("update.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&package).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("OEA", options).unwrap();
    zip.write_all(b"new executable").unwrap();
    zip.start_file("resources/new.txt", options).unwrap();
    zip.write_all(b"new resource").unwrap();
    zip.finish().unwrap();
    fs::write(root.join("OEA"), "old executable").unwrap();
    fs::create_dir_all(root.join("resources")).unwrap();
    fs::write(root.join("resources/old.txt"), "old resource").unwrap();
    package
}

#[tokio::test]
async fn failed_host_options_release_the_check_operation() {
    let updater = Updater::default();
    for _ in 0..2 {
        let error = updater
            .check("1.0.0", || {
                assert_eq!(updater.status().operation, UpdateOperationStatus::Checking);
                Err(Error::new(Reason::ProxyConfiguration))
            })
            .await
            .unwrap_err();
        assert_eq!(error.reason, Reason::ProxyConfiguration);
        assert_eq!(updater.status().operation, UpdateOperationStatus::Idle);
    }
}

#[test]
fn cancelling_the_host_package_picker_releases_installation_and_preserves_exclusion() {
    let root = tempfile::tempdir().unwrap();
    let target = InstallTarget::new(root.path(), "OEA").unwrap();
    let helper = HelperLaunch {
        source_executable: root.path().join("unused"),
        args: Vec::new(),
    };
    let updater = Updater::default();
    let result = updater
        .install_local(
            || {
                assert!(updater.is_installing());
                assert_eq!(
                    updater
                        .install(&target, || Ok(helper.clone()), |_| {})
                        .unwrap_err()
                        .reason,
                    Reason::Busy
                );
                Ok(None)
            },
            &target,
            || Ok(helper.clone()),
            |_| panic!("取消选择后不应准备安装"),
        )
        .unwrap();
    assert!(result.is_none());
    assert_eq!(updater.status().operation, UpdateOperationStatus::Idle);
}

#[test]
fn helper_spawn_failure_cancels_the_published_transaction_and_consumes_the_package() {
    let root = tempfile::tempdir().unwrap();
    let target = InstallTarget::new(root.path(), "OEA").unwrap();
    let package = full_package(root.path());
    let source = root.path().join("helper-source");
    fs::write(&source, "not an executable").unwrap();
    let helper = HelperLaunch {
        source_executable: source,
        args: vec!["host-defined-argument".into()],
    };
    let updater = Updater::default();
    let stages = RefCell::new(Vec::new());
    let error = updater
        .install_local(
            || Ok(Some(package.clone())),
            &target,
            || Ok(helper.clone()),
            |stage| stages.borrow_mut().push(stage),
        )
        .unwrap_err();

    assert_eq!(error.reason, Reason::HelperStart);
    assert_eq!(
        *stages.borrow(),
        [
            InstallStage::Preparing,
            InstallStage::Extracting,
            InstallStage::ApplyingFull,
            InstallStage::CleaningUp,
        ]
    );
    assert_eq!(updater.status().operation, UpdateOperationStatus::Idle);
    assert!(!package.exists());
    assert!(!root.path().join("cache/update/transaction.json").exists());
    assert!(!root.path().join("cache/update/candidate").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("OEA")).unwrap(),
        "old executable"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("resources/old.txt")).unwrap(),
        "old resource"
    );
    assert_eq!(
        complete_startup_transaction(&target, || panic!("已取消的事务不应提交资源")).unwrap(),
        StartupUpdateResult::NoTransaction
    );
}

#[test]
fn helper_request_requires_the_actual_workspace_executable_before_host_initialization() {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join(if cfg!(windows) {
        "cache/update/helper.exe"
    } else {
        "cache/update/helper"
    });
    fs::create_dir_all(helper.parent().unwrap()).unwrap();
    fs::write(&helper, "helper").unwrap();
    let other = root.path().join("other");
    fs::write(&other, "helper").unwrap();
    assert!(validate_helper_request(root.path().to_path_buf(), "OEA".into(), &other).is_err());
    let request =
        validate_helper_request(root.path().to_path_buf(), "OEA".into(), &helper).unwrap();
    assert_eq!(request.root(), root.path().canonicalize().unwrap());
    assert_eq!(
        request
            .run(|| panic!("无事务时不应提交 executable"))
            .unwrap(),
        oea_update::install::HelperResult::NoTransaction
    );
}
