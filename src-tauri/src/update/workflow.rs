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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Preparing,
    BootstrapReady,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub status: UpdateStatus,
    pub current_version: String,
    pub available_version: Option<String>,
    pub release_notes: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
    pub error: Option<String>,
}

impl UpdateSnapshot {
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

#[derive(Debug, Clone)]
pub struct BootstrapHandoff {
    pub bootstrap_path: PathBuf,
    pub portable_root: PathBuf,
    pub transaction_dir: PathBuf,
}

struct State {
    snapshot: UpdateSnapshot,
    available: Option<AvailableUpdate>,
}

pub struct UpdateManager {
    paths: AppPaths,
    state: Mutex<State>,
}

impl UpdateManager {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            state: Mutex::new(State {
                snapshot: UpdateSnapshot::idle(),
                available: None,
            }),
        }
    }

    pub fn snapshot(&self) -> UpdateSnapshot {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot
            .clone()
    }

    pub fn check(&self, config: &OeaConfig, mut emit: impl FnMut(UpdateSnapshot)) -> Result<()> {
        self.change_status(UpdateStatus::Checking, None, &mut emit);
        match source::check(config, env!("CARGO_PKG_VERSION")) {
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
        let result = self.prepare(config, current_exe, caller_pid, &available, &mut emit);
        if let Err(error) = &result {
            self.change_status(UpdateStatus::Failed, Some(format!("{error:#}")), &mut emit);
        }
        result
    }

    pub fn fail(&self, error: impl Into<String>, mut emit: impl FnMut(UpdateSnapshot)) {
        self.change_status(UpdateStatus::Failed, Some(error.into()), &mut emit);
    }

    fn prepare(
        &self,
        config: &OeaConfig,
        current_exe: &Path,
        caller_pid: u32,
        available: &AvailableUpdate,
        emit: &mut impl FnMut(UpdateSnapshot),
    ) -> Result<BootstrapHandoff> {
        let transaction_dir = self.paths.cache_dir().join("updates/current");
        fs::create_dir_all(&transaction_dir).context("创建更新事务目录失败")?;
        let transaction_path = transaction_dir.join(TRANSACTION_FILE);
        let mut transaction =
            recover_or_create(&transaction_path, available, env!("CARGO_PKG_VERSION"))?;
        transaction.caller_pid = Some(caller_pid);
        transaction.save(&transaction_path)?;

        let part_path = transaction_dir.join(PARTIAL_ARTIFACT);
        let artifact_path = transaction_dir.join(ARTIFACT);
        let has_verified_artifact = matches!(
            transaction.stage,
            TransactionStage::Verified
                | TransactionStage::Prepared
                | TransactionStage::BootstrapReady
        ) && artifact_path.is_file();
        if !has_verified_artifact {
            if transaction.stage != TransactionStage::Downloaded || !part_path.is_file() {
                transaction.stage = TransactionStage::Downloading;
                transaction.save(&transaction_path)?;
                self.change_status(UpdateStatus::Downloading, None, emit);
                let client = source::build_client(config)?;
                let last_emit = Mutex::new(Instant::now() - Duration::from_secs(1));
                let expected_total = transaction.expected_size;
                download::download(
                    &client,
                    &DownloadRequest {
                        url: transaction.artifact_url.clone(),
                        part_path: part_path.clone(),
                        validator: transaction.http_validator.clone(),
                    },
                    |validator, total, _resumed| {
                        transaction.http_validator = validator;
                        transaction.expected_size = transaction.expected_size.or(total);
                        transaction.save(&transaction_path)
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
                transaction.save(&transaction_path)?;
            }

            self.change_status(UpdateStatus::Verifying, None, emit);
            verify_sha256(&part_path, &transaction.expected_sha256)?;
            if artifact_path.exists() {
                fs::remove_file(&artifact_path).context("替换已下载归档失败")?;
            }
            fs::rename(&part_path, &artifact_path).context("发布已校验归档失败")?;
            transaction.stage = TransactionStage::Verified;
            transaction.save(&transaction_path)?;
        }

        let candidate_exe = transaction_dir.join(super::transaction::CANDIDATE_EXE);
        let published_assets = self
            .paths
            .root_dir()
            .join("assets")
            .join(format!("v{}", transaction.target_version));
        let already_prepared = matches!(
            transaction.stage,
            TransactionStage::Prepared | TransactionStage::BootstrapReady
        ) && candidate_exe.is_file()
            && published_assets.is_dir();
        if !already_prepared {
            self.change_status(UpdateStatus::Preparing, None, emit);
            prepare_full_package(
                self.paths.root_dir(),
                &transaction_dir,
                &transaction.target_version,
            )?;
            transaction.stage = TransactionStage::Prepared;
            transaction.save(&transaction_path)?;
        }

        let bootstrap_path = transaction_dir.join(BOOTSTRAP_EXE);
        fs::copy(current_exe, &bootstrap_path).context("复制 Bootstrap 可执行文件失败")?;
        transaction.stage = TransactionStage::BootstrapReady;
        transaction.save(&transaction_path)?;
        self.change_status(UpdateStatus::BootstrapReady, None, emit);

        Ok(BootstrapHandoff {
            bootstrap_path,
            portable_root: self.paths.root_dir().to_path_buf(),
            transaction_dir,
        })
    }

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

    fn set_progress(&self, progress: Progress, emit: &mut impl FnMut(UpdateSnapshot)) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.snapshot.downloaded_bytes = progress.downloaded_bytes;
        state.snapshot.total_bytes = progress.total_bytes;
        state.snapshot.bytes_per_second = progress.bytes_per_second;
        emit(state.snapshot.clone());
    }
}

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
}
