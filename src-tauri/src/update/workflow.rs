//! 自动更新的可移植工作流。
//!
//! `UpdateManager` 是 Rust 侧的事务所有者：前端只发送“检查”和“下载并安装”意图，
//! 本模块负责状态快照、下载、校验、解压、资源发布和 Bootstrap 交接。
//! Windows 文件替换被隔离在 `bootstrap::PlatformOps` 之后。

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{app_paths::AppPaths, config::OeaConfig};

use super::{
    download::{self, DownloadRequest, Progress},
    package::prepare_full_package,
    source::{self, AvailableUpdate},
    transaction::{
        ARTIFACT, BOOTSTRAP_EXE, PARTIAL_ARTIFACT, TRANSACTION_FILE, Transaction, TransactionStage,
    },
};

pub const UPDATE_EVENT: &str = "update-state-changed";

/// 前端可观察的更新状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStatus {
    /// 尚未检查。
    Idle,
    /// 正在获取少量版本元数据。
    Checking,
    /// 当前版本不低于来源最新版本。
    UpToDate,
    /// 已缓存一个可安装完整包的元数据。
    Available,
    /// 正在下载或续传完整包。
    Downloading,
    /// 正在验证完整包 SHA-256。
    Verifying,
    /// 正在解压候选程序并发布版本资源。
    Preparing,
    /// Bootstrap 已复制，可以退出当前应用完成替换。
    BootstrapReady,
    /// 最近一次检查或准备失败，`error` 包含原因。
    Failed,
}

/// Rust 更新工作流维护、由 Tauri 命令层发布给前端的完整更新快照。
///
/// 每个 `update-state-changed` 事件都携带全部字段；前端丢失事件后可调用
/// `update_get_snapshot` 命令重新同步。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    /// 当前状态机节点。
    pub status: UpdateStatus,
    /// 当前运行程序的内置版本。
    pub current_version: String,
    /// 检查成功且有更新时的新版本。
    pub available_version: Option<String>,
    /// 新版本 Markdown 更新日志。
    pub release_notes: Option<String>,
    /// 当前部分文件已写入的总字节数。
    pub downloaded_bytes: u64,
    /// 总大小未知时为 `None`，前端应显示不确定进度。
    pub total_bytes: Option<u64>,
    /// 本次运行新传输字节的平均速度。
    pub bytes_per_second: u64,
    /// 失败状态的人类可读错误链。
    pub error: Option<String>,
}

impl UpdateSnapshot {
    /// 创建尚未检查更新的初始快照。
    fn idle() -> Self {
        Self {
            status: UpdateStatus::Idle,
            current_version: env!("CARGO_PKG_VERSION").into(),
            available_version: None,
            release_notes: None,
            downloaded_bytes: 0,
            total_bytes: None,
            bytes_per_second: 0,
            error: None,
        }
    }
}

/// 主程序交给 Bootstrap 启动命令的三个绝对路径。
#[derive(Debug, Clone)]
pub struct BootstrapHandoff {
    /// 事务目录中的旧版本程序副本。
    pub bootstrap_path: PathBuf,
    /// 包含根入口和版本资源的便携目录。
    pub portable_root: PathBuf,
    /// 包含 transaction、candidate 与 bootstrap 的目录。
    pub transaction_dir: PathBuf,
}

/// 仅存在于当前进程内的更新状态。
struct State {
    /// 供 Tauri 命令层读取或发布的最新展示快照。
    snapshot: UpdateSnapshot,
    /// 最近检查得到的可信下载元数据；失败重检时必须清空。
    available: Option<AvailableUpdate>,
}

/// 串行拥有更新快照和当前候选元数据的工作流入口。
///
/// `Mutex` 让 Tauri 命令和进度回调可以共享状态；文件系统恢复信息则保存在事务中。
pub struct UpdateManager {
    paths: AppPaths,
    state: Mutex<State>,
}

impl UpdateManager {
    /// 为指定便携根目录创建空闲的更新管理器。
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            state: Mutex::new(State {
                snapshot: UpdateSnapshot::idle(),
                available: None,
            }),
        }
    }

    /// 返回当前完整快照的副本，供首次加载或事件丢失后恢复 UI。
    pub fn snapshot(&self) -> UpdateSnapshot {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .clone()
    }

    /// 使用已保存配置检查更新，并在每次状态变化时通过 `emit` 回调报告完整快照。
    ///
    /// Tauri 命令层负责把生产环境中的回调结果发布为 `update-state-changed` 事件。
    pub fn check(&self, config: &OeaConfig, mut emit: impl FnMut(UpdateSnapshot)) -> Result<()> {
        self.check_with(
            || source::check(config, env!("CARGO_PKG_VERSION")),
            &mut emit,
        )
    }

    /// 检查流程的可控来源实现。
    ///
    /// `lookup` 使测试可以模拟来源成功或失败；开始新检查时先清空旧元数据，
    /// 防止一次失败重检后仍安装旧 URL 和校验值。
    fn check_with(
        &self,
        lookup: impl FnOnce() -> Result<Option<AvailableUpdate>>,
        mut emit: impl FnMut(UpdateSnapshot),
    ) -> Result<()> {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.available = None;
            state.snapshot.status = UpdateStatus::Checking;
            state.snapshot.available_version = None;
            state.snapshot.release_notes = None;
            state.snapshot.downloaded_bytes = 0;
            state.snapshot.total_bytes = None;
            state.snapshot.bytes_per_second = 0;
            state.snapshot.error = None;
            emit(state.snapshot.clone());
        }
        match lookup() {
            Ok(Some(available)) => {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                state.snapshot.status = UpdateStatus::Available;
                state.snapshot.available_version = Some(available.version.clone());
                state.snapshot.release_notes = Some(available.release_notes.clone());
                state.snapshot.error = None;
                state.available = Some(available);
                emit(state.snapshot.clone());
                Ok(())
            }
            Ok(None) => {
                self.change_status(UpdateStatus::UpToDate, None, &mut emit);
                Ok(())
            }
            Err(error) => {
                self.change_status(UpdateStatus::Failed, Some(format!("{error:#}")), &mut emit);
                Err(error)
            }
        }
    }

    /// 下载、校验并准备当前 `AvailableUpdate`，返回 Bootstrap 交接参数。
    ///
    /// 本函数不退出进程，也不替换根入口；这些副作用由 Tauri 命令和 Bootstrap 完成。
    pub fn download_and_prepare(
        &self,
        config: &OeaConfig,
        current_exe: &Path,
        caller_pid: u32,
        mut emit: impl FnMut(UpdateSnapshot),
    ) -> Result<BootstrapHandoff> {
        let available = self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .available
            .clone()
            .context("没有可安装的更新")?;
        let result = (|| {
            let transaction_dir = self.paths.cache_dir().join("updates/current");
            fs::create_dir_all(&transaction_dir).context("创建更新事务目录失败")?;
            let transaction_path = transaction_dir.join(TRANSACTION_FILE);
            let mut transaction =
                recover_or_create(&transaction_path, &available, env!("CARGO_PKG_VERSION"))?;
            transaction.caller_pid = Some(caller_pid);
            transaction.save(&transaction_path)?;

            self.download(
                config,
                &transaction_dir,
                &transaction_path,
                &mut transaction,
                &mut emit,
            )?;
            self.prepare(
                current_exe,
                &transaction_dir,
                &transaction_path,
                &mut transaction,
                &mut emit,
            )
        })();
        if let Err(error) = &result {
            self.change_status(UpdateStatus::Failed, Some(format!("{error:#}")), &mut emit);
        }
        result
    }

    /// 把 Tauri 命令层发生的错误写入管理器，并立即通过 `emit` 回调报告失败快照。
    pub fn fail(&self, error: impl Into<String>, mut emit: impl FnMut(UpdateSnapshot)) {
        self.change_status(UpdateStatus::Failed, Some(error.into()), &mut emit);
    }

    /// 下载完整包并把事务推进到 `Downloaded`；已下载或已校验的归档直接复用。
    fn download(
        &self,
        config: &OeaConfig,
        transaction_dir: &Path,
        transaction_path: &Path,
        transaction: &mut Transaction,
        emit: &mut impl FnMut(UpdateSnapshot),
    ) -> Result<()> {
        let part_path = transaction_dir.join(PARTIAL_ARTIFACT);
        let artifact_path = transaction_dir.join(ARTIFACT);
        let has_verified_artifact = matches!(
            transaction.stage,
            TransactionStage::Verified
                | TransactionStage::Prepared
                | TransactionStage::BootstrapReady
        ) && artifact_path.is_file();
        if has_verified_artifact
            || (transaction.stage == TransactionStage::Downloaded && part_path.is_file())
        {
            return Ok(());
        }

        transaction.stage = TransactionStage::Downloading;
        transaction.save(transaction_path)?;
        self.change_status(UpdateStatus::Downloading, None, emit);
        let client = source::build_client(config)?;
        // 下载器逐块报告；工作流最多约每 150 ms 通过 `emit` 回调报告一次完整快照。
        let last_emit = Mutex::new(Instant::now() - Duration::from_secs(1));
        let expected_total = transaction.expected_size;
        download::download(
            &client,
            &DownloadRequest {
                url: transaction.artifact_url.clone(),
                part_path,
                validator: transaction.http_validator.clone(),
            },
            |validator, total, _resumed| {
                transaction.http_validator = validator;
                transaction.expected_size = transaction.expected_size.or(total);
                transaction.save(transaction_path)
            },
            |mut progress| {
                progress.total_bytes = progress.total_bytes.or(expected_total);
                let mut last = last_emit.lock().unwrap_or_else(|e| e.into_inner());
                if last.elapsed() >= Duration::from_millis(150)
                    || progress.total_bytes == Some(progress.downloaded_bytes)
                {
                    self.set_progress(progress, emit);
                    *last = Instant::now();
                }
            },
        )?;
        transaction.stage = TransactionStage::Downloaded;
        transaction.save(transaction_path)
    }

    /// 校验已下载归档、发布版本资源并准备 Bootstrap 交接参数。
    fn prepare(
        &self,
        current_exe: &Path,
        transaction_dir: &Path,
        transaction_path: &Path,
        transaction: &mut Transaction,
        emit: &mut impl FnMut(UpdateSnapshot),
    ) -> Result<BootstrapHandoff> {
        let part_path = transaction_dir.join(PARTIAL_ARTIFACT);
        let artifact_path = transaction_dir.join(ARTIFACT);
        let has_verified_artifact = matches!(
            transaction.stage,
            TransactionStage::Verified
                | TransactionStage::Prepared
                | TransactionStage::BootstrapReady
        ) && artifact_path.is_file();
        if !has_verified_artifact {
            self.change_status(UpdateStatus::Verifying, None, emit);
            verify_sha256(&part_path, &transaction.expected_sha256)?;
            if artifact_path.exists() {
                fs::remove_file(&artifact_path).context("替换已下载归档失败")?;
            }
            fs::rename(&part_path, &artifact_path).context("发布已校验归档失败")?;
            transaction.stage = TransactionStage::Verified;
            transaction.save(transaction_path)?;
        }

        let candidate_exe = transaction_dir.join(super::transaction::CANDIDATE_EXE);
        let published_assets = self
            .paths
            .root_dir()
            .join("assets")
            .join(format!("v{}", transaction.target_version));
        // 资源目录一旦整体发布即视为完整；第一阶段按规格不重新校验其内容。
        let already_prepared = matches!(
            transaction.stage,
            TransactionStage::Prepared | TransactionStage::BootstrapReady
        ) && candidate_exe.is_file()
            && published_assets.is_dir();
        if !already_prepared {
            self.change_status(UpdateStatus::Preparing, None, emit);
            prepare_full_package(
                self.paths.root_dir(),
                transaction_dir,
                &transaction.target_version,
            )?;
            transaction.stage = TransactionStage::Prepared;
            transaction.save(transaction_path)?;
        }

        let bootstrap_path = transaction_dir.join(BOOTSTRAP_EXE);
        fs::copy(current_exe, &bootstrap_path).context("复制 Bootstrap 可执行文件失败")?;
        transaction.stage = TransactionStage::BootstrapReady;
        transaction.save(transaction_path)?;
        self.change_status(UpdateStatus::BootstrapReady, None, emit);

        Ok(BootstrapHandoff {
            bootstrap_path,
            portable_root: self.paths.root_dir().to_path_buf(),
            transaction_dir: transaction_dir.to_path_buf(),
        })
    }

    /// 修改阶段或错误，并立即通过 `emit` 回调报告完整快照。
    fn change_status(
        &self,
        status: UpdateStatus,
        error: Option<String>,
        emit: &mut impl FnMut(UpdateSnapshot),
    ) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.snapshot.status = status;
        state.snapshot.error = error;
        if status != UpdateStatus::Downloading {
            state.snapshot.bytes_per_second = 0;
        }
        emit(state.snapshot.clone());
    }

    /// 合并下载器进度到管理器快照，并调用 `emit` 回调报告完整快照。
    fn set_progress(&self, progress: Progress, emit: &mut impl FnMut(UpdateSnapshot)) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.snapshot.downloaded_bytes = progress.downloaded_bytes;
        state.snapshot.total_bytes = progress.total_bytes;
        state.snapshot.bytes_per_second = progress.bytes_per_second;
        emit(state.snapshot.clone());
    }
}

/// 复用与本次来源元数据完全匹配的事务，否则创建新事务。
///
/// 版本、URL 或 SHA-256 任一变化都意味着旧的部分文件不能再被当前事务信任。
fn recover_or_create(
    path: &Path,
    available: &AvailableUpdate,
    source_version: &str,
) -> Result<Transaction> {
    if let Ok(existing) = Transaction::load(path) {
        if existing.source_version == source_version
            && existing.target_version == available.version
            && existing.artifact_url == available.artifact_url
            && existing.expected_sha256 == available.sha256
        {
            return Ok(existing);
        }
    }
    // 固定事务目录可能仍有旧文件；新事务不带 validator，下载会截断部分文件，
    // 后续阶段也会替换其他固定文件，因此不需要先清空整个目录。
    let transaction_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("系统时间早于 Unix epoch")?
        .as_millis()
        .to_string();
    Ok(Transaction {
        schema_version: 1,
        transaction_id,
        stage: TransactionStage::Downloading,
        source_version: source_version.into(),
        target_version: available.version.clone(),
        download_source: available.source,
        artifact_url: available.artifact_url.clone(),
        expected_sha256: available.sha256.clone(),
        expected_size: available.size,
        http_validator: None,
        caller_pid: None,
    })
}

/// 流式计算文件 SHA-256，并与来源提供的期望值比较。
///
/// 校验失败只返回错误，不移动部分文件，也不发布任何候选资源。
fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = fs::File::open(path).context("打开待校验归档失败")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).context("读取待校验归档失败")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != expected.to_ascii_lowercase() {
        bail!("更新归档 SHA-256 不匹配");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, thread};

    use sha2::{Digest, Sha256};
    use tiny_http::{Header, Response, Server};

    use crate::{app_paths::AppPaths, config::OeaConfig};

    use super::{UpdateManager, UpdateStatus, verify_sha256};
    use crate::update::{
        source::AvailableUpdate,
        transaction::{DownloadSource, TRANSACTION_FILE, Transaction, TransactionStage},
    };

    fn full_package(version: &str) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, contents) in [
            ("OEA.exe".to_string(), b"new exe".as_slice()),
            (
                format!("assets/v{version}/models/model.onnx"),
                b"model".as_slice(),
            ),
            (
                format!("assets/v{version}/resources/data.json"),
                b"data".as_slice(),
            ),
        ] {
            archive
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(contents).unwrap();
        }
        archive.finish().unwrap().into_inner()
    }

    #[test]
    fn sha256_mismatch_is_rejected_without_changing_the_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("artifact.zip.part");
        fs::write(&path, b"payload").unwrap();

        assert!(verify_sha256(&path, &"0".repeat(64)).is_err());
        assert_eq!(fs::read(path).unwrap(), b"payload");
    }

    #[test]
    fn update_manager_prepares_a_complete_portable_handoff() {
        let package = full_package("0.2.0");
        let sha256 = Sha256::digest(&package)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let package_for_server = package.clone();
        let server_handle = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(
                    Response::from_data(package_for_server)
                        .with_header(Header::from_bytes(b"ETag", b"\"package-v1\"").unwrap()),
                )
                .unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        fs::create_dir_all(&root).unwrap();
        let current_exe = root.join("OEA.exe");
        fs::write(&current_exe, b"old exe").unwrap();
        let manager = UpdateManager::new(AppPaths::for_build(
            root.clone(),
            env!("CARGO_PKG_VERSION"),
            false,
        ));
        manager.state.lock().unwrap().available = Some(AvailableUpdate {
            version: "0.2.0".into(),
            release_notes: "notes".into(),
            artifact_url: url,
            sha256,
            size: Some(package.len() as u64),
            source: DownloadSource::Github,
        });
        let mut snapshots = Vec::new();

        let handoff = manager
            .download_and_prepare(&OeaConfig::default(), &current_exe, 1234, |snapshot| {
                snapshots.push(snapshot)
            })
            .unwrap();
        server_handle.join().unwrap();

        let transaction_dir = root.join("cache/updates/current");
        let transaction = Transaction::load(&transaction_dir.join(TRANSACTION_FILE)).unwrap();
        assert_eq!(transaction.stage, TransactionStage::BootstrapReady);
        assert_eq!(transaction.caller_pid, Some(1234));
        assert_eq!(
            fs::read(root.join("assets/v0.2.0/resources/data.json")).unwrap(),
            b"data"
        );
        assert_eq!(
            fs::read(transaction_dir.join("candidate/OEA.exe")).unwrap(),
            b"new exe"
        );
        assert_eq!(
            fs::read(transaction_dir.join("bootstrap.exe")).unwrap(),
            b"old exe"
        );
        assert_eq!(handoff.portable_root, root);
        assert_eq!(handoff.transaction_dir, transaction_dir);
        assert_eq!(
            handoff.bootstrap_path,
            handoff.transaction_dir.join("bootstrap.exe")
        );
        assert_eq!(fs::read(&current_exe).unwrap(), b"old exe");
        let statuses = snapshots
            .iter()
            .map(|snapshot| snapshot.status)
            .collect::<Vec<_>>();
        for expected in [
            UpdateStatus::Downloading,
            UpdateStatus::Verifying,
            UpdateStatus::Preparing,
            UpdateStatus::BootstrapReady,
        ] {
            assert!(statuses.contains(&expected));
        }
    }

    #[test]
    fn failed_recheck_discards_previously_available_update() {
        let manager = UpdateManager::new(AppPaths::for_build(
            "/portable",
            env!("CARGO_PKG_VERSION"),
            false,
        ));
        {
            let mut state = manager.state.lock().unwrap();
            state.available = Some(AvailableUpdate {
                version: "0.2.0".into(),
                release_notes: "old notes".into(),
                artifact_url: "https://old.example/OEA.zip".into(),
                sha256: "a".repeat(64),
                size: Some(42),
                source: DownloadSource::Github,
            });
            state.snapshot.available_version = Some("0.2.0".into());
            state.snapshot.release_notes = Some("old notes".into());
        }

        assert!(
            manager
                .check_with(|| anyhow::bail!("source failed"), |_| {})
                .is_err()
        );

        let state = manager.state.lock().unwrap();
        assert!(state.available.is_none());
        assert_eq!(state.snapshot.status, UpdateStatus::Failed);
        assert_eq!(state.snapshot.available_version, None);
        assert_eq!(state.snapshot.release_notes, None);
    }

    #[test]
    fn retry_after_asset_publication_reuses_prepared_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction_dir = root.join("cache/updates/current");
        let published_assets = root.join("assets/v0.2.0");
        fs::create_dir_all(transaction_dir.join("candidate")).unwrap();
        fs::create_dir_all(&published_assets).unwrap();
        fs::write(transaction_dir.join("artifact.zip"), b"verified archive").unwrap();
        fs::write(transaction_dir.join("candidate/OEA.exe"), b"prepared exe").unwrap();
        fs::write(published_assets.join("marker"), b"published assets").unwrap();
        let current_exe = root.join("OEA.exe");
        fs::write(&current_exe, b"old exe").unwrap();
        let available = AvailableUpdate {
            version: "0.2.0".into(),
            release_notes: "notes".into(),
            artifact_url: "https://unused.example/OEA.zip".into(),
            sha256: "a".repeat(64),
            size: Some(16),
            source: DownloadSource::Github,
        };
        Transaction {
            schema_version: 1,
            transaction_id: "existing".into(),
            stage: TransactionStage::Prepared,
            source_version: env!("CARGO_PKG_VERSION").into(),
            target_version: available.version.clone(),
            download_source: available.source,
            artifact_url: available.artifact_url.clone(),
            expected_sha256: available.sha256.clone(),
            expected_size: available.size,
            http_validator: None,
            caller_pid: Some(1),
        }
        .save(&transaction_dir.join(TRANSACTION_FILE))
        .unwrap();
        let manager = UpdateManager::new(AppPaths::for_build(
            root.clone(),
            env!("CARGO_PKG_VERSION"),
            false,
        ));
        manager.state.lock().unwrap().available = Some(available);
        let mut snapshots = Vec::new();

        manager
            .download_and_prepare(&OeaConfig::default(), &current_exe, 4321, |snapshot| {
                snapshots.push(snapshot)
            })
            .unwrap();

        assert_eq!(
            fs::read(published_assets.join("marker")).unwrap(),
            b"published assets"
        );
        assert_eq!(
            fs::read(transaction_dir.join("candidate/OEA.exe")).unwrap(),
            b"prepared exe"
        );
        assert_eq!(
            fs::read(transaction_dir.join("bootstrap.exe")).unwrap(),
            b"old exe"
        );
        assert_eq!(
            snapshots
                .iter()
                .map(|snapshot| snapshot.status)
                .collect::<Vec<_>>(),
            [UpdateStatus::BootstrapReady]
        );
    }
}
