//! 自动更新的 candidate/transaction 安装核心。
//!
//! 更新事务只有一个发布标记：`cache/update/transaction.json`。candidate 在标记创建
//! 前完整构造，helper 只提交 exe，v2 启动阶段提交 resources；candidate 内尚未移动
//! 的条目代表事务进度。这里不记录版本号、不做启动哈希扫描，也不提供旧版本回滚。

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use serde::Serialize;
use tauri::Emitter;

use crate::app_paths::AppPaths;

mod candidate;
mod helper;
mod startup;
mod workspace;

use candidate::{PackageKind, extract_package_zip, prepare_candidate};
#[cfg(test)]
use helper::HelperResult;
use helper::spawn_helper;
pub use helper::{helper_request_from_args, run_helper};
pub use startup::{StartupUpdateResult, complete_startup_transaction};
pub use workspace::UpdateWorkspace;

static STARTUP_UPDATE_RESULT: OnceLock<Mutex<Option<StartupUpdateResult>>> = OnceLock::new();

fn startup_result_slot() -> &'static Mutex<Option<StartupUpdateResult>> {
    STARTUP_UPDATE_RESULT.get_or_init(|| Mutex::new(None))
}

/// 记录应用启动早期完成的资源事务结果，供 Tauri 前端在初始化后消费。
pub fn record_startup_update_result(result: StartupUpdateResult) {
    if result == StartupUpdateResult::Completed {
        *startup_result_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(result);
    }
}

/// 取出并清除本进程启动早期的更新结果。
#[tauri::command]
pub fn consume_startup_update_result() -> Option<StartupUpdateResult> {
    startup_result_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum InstallStage {
    Preparing,
    Extracting,
    ApplyingIncremental,
    ApplyingFull,
    CleaningUp,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct InstallStageEvent {
    stage: InstallStage,
}

fn emit_install_stage(app: &tauri::AppHandle, stage: InstallStage) {
    let _ = app.emit("update-install-stage", InstallStageEvent { stage });
}

fn validate_download_package(paths: &AppPaths, package_path: &str) -> Result<PathBuf, String> {
    let downloads = paths.cache_dir().join("downloads");
    let canonical_downloads = downloads
        .canonicalize()
        .map_err(|error| format!("无法定位更新下载目录: {error}"))?;
    let path = Path::new(package_path);
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("更新包不存在: {error}"))?;
    if !canonical_path.starts_with(&canonical_downloads) || !canonical_path.is_file() {
        return Err(format!("更新包不在 cache/downloads 内: {package_path}"));
    }
    Ok(canonical_path)
}

/// 检查 pending metadata 指向的 zip 是否仍在受控 downloads 目录内。
#[tauri::command]
pub fn pending_package_exists(package_path: String) -> bool {
    let Ok(paths) = AppPaths::new() else {
        return false;
    };
    validate_download_package(&paths, &package_path).is_ok()
}

/// 构造 candidate、原子发布 transaction、复制并启动 helper，然后请求当前 v1 退出。
///
/// 这是前端唯一需要调用的安装接口。debug 构建禁止触碰项目根目录；集成测试应直接
/// 使用 [`prepare_candidate`]、[`run_helper`] 和 [`complete_startup_transaction`] 的临时
/// workspace seam。
#[tauri::command]
pub fn install_update(
    manager: tauri::State<'_, super::UpdateManager>,
    app: tauri::AppHandle,
    package_path: String,
) -> Result<(), String> {
    manager.begin_install().map_err(|error| error.to_string())?;
    let result = install_update_inner(app, package_path);
    if result.is_err() {
        let _ = manager.finish_install();
    }
    result
}

fn install_update_inner(app: tauri::AppHandle, package_path: String) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("开发构建禁止执行真实自更新，请使用临时目录集成测试".to_string());
    }

    let paths = AppPaths::new().map_err(|error| format!("无法定位应用根目录: {error}"))?;
    let package_zip = validate_download_package(&paths, &package_path)?;
    let workspace = UpdateWorkspace::for_current_executable(paths.root_dir())
        .map_err(|error| format!("无法确定应用 executable name: {error}"))?;
    // v1 能进入正常 GUI 说明 helper 没有持锁；新的安装始终抛弃旧 candidate，绝不
    // 复用已经消费过的 ZIP 或半途事务。
    workspace.remove_transaction_workspace()?;
    let package_dir = workspace.package_path();

    emit_install_stage(&app, InstallStage::Preparing);
    emit_install_stage(&app, InstallStage::Extracting);
    if let Err(error) = extract_package_zip(&package_zip, &package_dir) {
        let _ = fs::remove_file(&package_zip);
        let _ = workspace.remove_transaction_workspace();
        return Err(error);
    }

    let kind = match PackageKind::detect(&package_dir) {
        Ok(kind) => kind,
        Err(error) => {
            let _ = fs::remove_file(&package_zip);
            let _ = workspace.remove_transaction_workspace();
            return Err(error);
        }
    };
    emit_install_stage(
        &app,
        match kind {
            PackageKind::Full => InstallStage::ApplyingFull,
            PackageKind::Incremental => InstallStage::ApplyingIncremental,
        },
    );
    if let Err(error) = prepare_candidate(&workspace, &package_dir) {
        let _ = fs::remove_file(&package_zip);
        let _ = workspace.remove_transaction_workspace();
        return Err(error);
    }

    // transaction 已证明 candidate 完整；zip 从此不能被复用，删除失败则不启动 helper。
    if let Err(error) = fs::remove_file(&package_zip) {
        let _ = workspace.remove_transaction_workspace();
        return Err(format!("删除已消费的更新包失败: {error}"));
    }
    emit_install_stage(&app, InstallStage::CleaningUp);
    let _ = fs::remove_dir_all(&package_dir);

    if let Err(error) = spawn_helper(&workspace) {
        let _ = fs::remove_file(&package_zip);
        return Err(error);
    }

    // helper 已经独立接管事务；当前进程必须退出，释放 Windows 对根 exe 的占用。
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        ffi::OsString,
        fs,
        io::Write,
        path::{Path, PathBuf},
        process::Command,
    };

    use super::helper::{EXECUTABLE_NAME_ARGUMENT, HELPER_ARGUMENT, ROOT_ARGUMENT};
    use super::*;

    fn temp_root(name: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("oea-update-{name}-"))
            .tempdir()
            .unwrap()
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn workspace(root: &Path) -> UpdateWorkspace {
        UpdateWorkspace::with_executable_name(root, "OEA")
    }

    fn make_full_package(root: &Path) -> PathBuf {
        let package = root.join("package");
        write_file(&package.join("OEA"), "v2-executable");
        write_file(&package.join("resources/data/new.txt"), "v2-resource");
        package
    }

    fn make_full_binary_package(root: &Path) -> PathBuf {
        let package = root.join("binary-package");
        fs::create_dir_all(&package).unwrap();
        fs::copy(env::current_exe().unwrap(), package.join("OEA")).unwrap();
        write_file(&package.join("resources/data/new.txt"), "v2-resource");
        package
    }

    fn copy_test_binary(path: &Path) {
        fs::copy(env::current_exe().unwrap(), path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(env::current_exe().unwrap())
                .unwrap()
                .permissions()
                .mode();
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }
    }

    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, content) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    #[cfg(unix)]
    fn make_executable_zip(path: &Path) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(
                "OEA",
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
        writer.write_all(b"executable").unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn full_prepare_publishes_complete_candidate_before_transaction() {
        let root = temp_root("full");
        let package = make_full_package(root.path());
        let workspace = workspace(root.path());

        let kind = prepare_candidate(&workspace, &package).unwrap();

        assert_eq!(kind, PackageKind::Full);
        assert_eq!(
            fs::read_to_string(workspace.candidate_path().join("OEA")).unwrap(),
            "v2-executable"
        );
        assert_eq!(
            fs::read_to_string(workspace.candidate_path().join("resources/data/new.txt")).unwrap(),
            "v2-resource"
        );
        assert_eq!(
            fs::read_to_string(workspace.transaction_path()).unwrap(),
            "{\"schema_version\":1}\n"
        );
    }

    #[test]
    fn incremental_prepare_applies_all_five_changes_from_read_only_baseline() {
        let root = temp_root("incremental");
        let workspace = workspace(root.path());
        write_file(&workspace.executable_path(), "v1-executable");
        write_file(&workspace.resources_path().join("keep.txt"), "keep");
        write_file(&workspace.resources_path().join("old.txt"), "old");
        write_file(
            &workspace.resources_path().join("old-dir/item.txt"),
            "old-dir",
        );

        let package = root.path().join("package");
        write_file(&package.join("OEA"), "unused-exe-change");
        write_file(&package.join("resources/changed.txt"), "changed");
        write_file(&package.join("resources/added-dir/new.txt"), "new");
        write_file(
            &package.join("changes.json"),
            r#"{
                "added": ["resources/added-dir/new.txt"],
                "modified": ["resources/changed.txt"],
                "deleted": ["resources/old.txt", "resources/old-dir/item.txt"],
                "added_dir": ["resources/added-dir"],
                "deleted_dir": ["resources/old-dir"]
            }"#,
        );

        assert_eq!(
            prepare_candidate(&workspace, &package).unwrap(),
            PackageKind::Incremental
        );
        let candidate = workspace.candidate_path();
        assert!(candidate.join("OEA").is_file());
        assert!(candidate.join("resources/keep.txt").is_file());
        assert_eq!(
            fs::read_to_string(candidate.join("resources/changed.txt")).unwrap(),
            "changed"
        );
        assert_eq!(
            fs::read_to_string(candidate.join("resources/added-dir/new.txt")).unwrap(),
            "new"
        );
        assert!(!candidate.join("resources/old.txt").exists());
        assert!(!candidate.join("resources/old-dir").exists());
        assert!(
            workspace
                .baseline_path()
                .join("resources/old.txt")
                .is_file()
        );
    }

    #[test]
    fn unsafe_incremental_path_does_not_publish_transaction() {
        let root = temp_root("unsafe-path");
        let workspace = workspace(root.path());
        write_file(&workspace.executable_path(), "v1");
        write_file(&workspace.resources_path().join("keep.txt"), "keep");
        let package = root.path().join("package");
        write_file(
            &package.join("changes.json"),
            r#"{"added":["config/secret"]}"#,
        );

        let error = prepare_candidate(&workspace, &package).unwrap_err();
        assert!(error.contains("只能指向"));
        assert!(!workspace.transaction_path().exists());
        assert!(!workspace.candidate_path().exists());
    }

    #[test]
    fn startup_moves_resources_and_removes_transaction_at_commit_point() {
        let root = temp_root("startup");
        let workspace = workspace(root.path());
        write_file(&workspace.executable_path(), "v1");
        write_file(&workspace.resources_path().join("old.txt"), "old");
        let package = make_full_package(root.path());
        prepare_candidate(&workspace, &package).unwrap();
        fs::remove_file(workspace.candidate_path().join("OEA")).unwrap();

        assert_eq!(
            complete_startup_transaction(&workspace).unwrap(),
            StartupUpdateResult::Completed
        );
        assert_eq!(
            fs::read_to_string(workspace.resources_path().join("data/new.txt")).unwrap(),
            "v2-resource"
        );
        assert!(!workspace.transaction_path().exists());
        assert!(!workspace.discard_path().exists());
    }

    #[test]
    fn startup_leaves_old_version_running_when_helper_has_not_committed_exe() {
        let root = temp_root("waiting-helper");
        let workspace = workspace(root.path());
        write_file(&workspace.executable_path(), "v1");
        write_file(&workspace.resources_path().join("old.txt"), "old");
        let package = make_full_package(root.path());
        prepare_candidate(&workspace, &package).unwrap();

        assert_eq!(
            complete_startup_transaction(&workspace).unwrap(),
            StartupUpdateResult::WaitingForHelper
        );
        assert_eq!(
            fs::read_to_string(workspace.executable_path()).unwrap(),
            "v1"
        );
        assert!(workspace.transaction_path().exists());
    }

    #[test]
    fn transaction_lock_is_exclusive_and_released_with_handle() {
        let root = temp_root("lock");
        let workspace = workspace(root.path());
        let first = workspace.try_lock().unwrap().expect("first lock");
        assert!(workspace.try_lock().unwrap().is_none());
        drop(first);
        assert!(workspace.try_lock().unwrap().is_some());
    }

    #[test]
    fn helper_arguments_carry_root_and_target_executable_name() {
        let parsed = helper_request_from_args([
            OsString::from("OEA"),
            OsString::from(HELPER_ARGUMENT),
            OsString::from(ROOT_ARGUMENT),
            OsString::from("/tmp/update-root"),
            OsString::from(EXECUTABLE_NAME_ARGUMENT),
            OsString::from("oea"),
        ])
        .unwrap();
        assert_eq!(parsed.0, PathBuf::from("/tmp/update-root"));
        assert_eq!(parsed.1, OsString::from("oea"));
    }

    /// 这个测试函数也作为 test harness 中的真实 helper 子进程入口。
    #[test]
    fn helper_process_entrypoint() {
        let Some(root) = env::var_os("OEA_TEST_HELPER_ROOT") else {
            return;
        };
        let workspace = UpdateWorkspace::with_executable_name(root, "OEA");
        assert_eq!(
            run_helper(&workspace).unwrap(),
            HelperResult::ExecutableCommitted
        );
    }

    /// 复制到临时根目录的 v2 binary 作为真实 startup 子进程入口。
    #[test]
    fn startup_process_entrypoint() {
        let Some(root) = env::var_os("OEA_TEST_STARTUP_ROOT") else {
            return;
        };
        let workspace = UpdateWorkspace::with_executable_name(root, "OEA");
        assert_eq!(
            complete_startup_transaction(&workspace).unwrap(),
            StartupUpdateResult::Completed
        );
    }

    #[test]
    fn end_to_end_helper_subprocess_then_v2_startup_in_temp_root() {
        let root = temp_root("e2e");
        let workspace = workspace(root.path());
        copy_test_binary(&workspace.executable_path());
        write_file(&workspace.resources_path().join("old.txt"), "v1-resource");
        let package = make_full_binary_package(root.path());
        let expected_v2_executable = fs::read(package.join("OEA")).unwrap();
        prepare_candidate(&workspace, &package).unwrap();

        // v1 退出前把自身复制到 cache/update/helper；helper 从副本运行，不能直接执行
        // 将被替换的根目录 exe。根目录和 cache 均为临时目录。
        let helper_copy = workspace
            .prepare_helper_copy(&env::current_exe().unwrap())
            .unwrap();
        let status = Command::new(helper_copy)
            .arg("update::install::tests::helper_process_entrypoint")
            .arg("--exact")
            .arg("--nocapture")
            .env("OEA_TEST_HELPER_ROOT", root.path())
            .env("OEA_UPDATE_SILENT", "1")
            .status()
            .unwrap();
        assert!(status.success(), "helper 子进程失败: {status}");
        assert_eq!(
            fs::read(workspace.executable_path()).unwrap(),
            expected_v2_executable
        );
        assert!(!workspace.candidate_path().join("OEA").exists());
        assert!(workspace.transaction_path().exists());
        assert!(workspace.helper_path().exists());

        // 现在让根目录中的 v2 binary 真实启动并完成 resources 事务。helper/v2 之间
        // 只通过 transaction/candidate 状态交接。
        let status = Command::new(workspace.executable_path())
            .arg("update::install::tests::startup_process_entrypoint")
            .arg("--exact")
            .arg("--nocapture")
            .env("OEA_TEST_STARTUP_ROOT", root.path())
            .env("OEA_UPDATE_SILENT", "1")
            .status()
            .unwrap();
        assert!(status.success(), "v2 startup 子进程失败: {status}");
        assert_eq!(
            fs::read_to_string(workspace.resources_path().join("data/new.txt")).unwrap(),
            "v2-resource"
        );
        assert!(!workspace.transaction_path().exists());
        assert!(!workspace.helper_path().exists());
    }

    #[test]
    fn zip_extraction_keeps_unsafe_entries_outside_package() {
        let root = temp_root("zip");
        let zip_path = root.path().join("package.zip");
        let output = root.path().join("package");
        make_zip(
            &zip_path,
            &[("resources/a.txt", "a"), ("../outside", "bad")],
        );

        // 复用旧 zip 解包行为的高层约定；candidate 只接受解压后的 package 目录。
        assert!(crate::update::install::extract_package_zip(&zip_path, &output).is_err());
        assert!(output.join("resources/a.txt").exists());
        assert!(!root.path().join("outside").exists());
    }

    #[cfg(unix)]
    #[test]
    fn zip_extraction_restores_executable_mode() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("zip-mode");
        let zip_path = root.path().join("package.zip");
        let output = root.path().join("package");
        make_executable_zip(&zip_path);

        extract_package_zip(&zip_path, &output).unwrap();

        let mode = fs::metadata(output.join("OEA"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}
