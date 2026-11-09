use std::{
    cell::Cell,
    ffi::OsStr,
    fs::{self, File},
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(15);
pub const APP: &str = if cfg!(windows) {
    "test host.exe"
} else {
    "test host"
};

/// 仅驱动公开安装入口和真实进程。文件树修改只用于持久化协议的恢复现场。
pub struct Installation {
    pub root: tempfile::TempDir,
    listener: TcpListener,
    sequence: Cell<usize>,
}

impl Installation {
    pub fn new() -> Self {
        Self::with_prefix(OsStr::new("update-install-"))
    }

    pub fn with_prefix(prefix: &OsStr) -> Self {
        let root = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
        fs::copy(env!("CARGO_BIN_EXE_update-host-v1"), root.path().join(APP)).unwrap();
        fs::create_dir(root.path().join("resources")).unwrap();
        fs::write(root.path().join("resources/old.txt"), "old resource").unwrap();
        fs::write(root.path().join("settings.json"), "user settings").unwrap();
        fs::write(root.path().join("resources/keep.txt"), "unchanged").unwrap();
        fs::write(root.path().join("resources/changed.txt"), "before").unwrap();
        fs::create_dir(root.path().join("resources/old-dir")).unwrap();
        fs::write(root.path().join("resources/old-dir/item.txt"), "removed").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            root,
            listener,
            sequence: Cell::new(0),
        }
    }

    pub fn full_package(&self) -> PathBuf {
        self.package(true, &[("resources/new.txt", b"new resource")])
    }

    pub fn package(&self, executable: bool, entries: &[(&str, &[u8])]) -> PathBuf {
        let downloads = self.root.path().join("cache/downloads");
        fs::create_dir_all(&downloads).unwrap();
        let package = downloads.join("update.zip");
        let mut zip = zip::ZipWriter::new(File::create(&package).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        if executable {
            zip.start_file(APP, options).unwrap();
            io::copy(
                &mut File::open(env!("CARGO_BIN_EXE_update-host-v2")).unwrap(),
                &mut zip,
            )
            .unwrap();
        }
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        package
    }

    pub fn install(&self, package: &Path, mode: &str) -> Process {
        let mut command = Command::new(self.root.path().join(APP));
        command.arg("install").arg(package).arg(mode);
        self.spawn(command, "install:v1")
    }

    pub fn start(&self, result: &str, version: u8) -> bool {
        let mut process = self.start_process(version);
        let event = process.next();
        let progress = event == "resources";
        let result_event = if progress { process.next() } else { event };
        assert_eq!(result_event, format!("startup:{result}"));
        process.wait(true);
        progress
    }

    pub fn start_process(&self, version: u8) -> Process {
        let mut command = Command::new(self.root.path().join(APP));
        command.arg("start");
        self.spawn(command, &format!("start:v{version}"))
    }

    pub fn assert_old(&self) {
        assert_eq!(
            fs::read(self.root.path().join(APP)).unwrap(),
            fs::read(env!("CARGO_BIN_EXE_update-host-v1")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(self.root.path().join("resources/old.txt")).unwrap(),
            "old resource"
        );
        assert_eq!(
            fs::read_to_string(self.root.path().join("settings.json")).unwrap(),
            "user settings"
        );
    }

    pub fn helper_path(&self) -> PathBuf {
        self.root.path().join(if cfg!(windows) {
            "cache/update/helper.exe"
        } else {
            "cache/update/helper"
        })
    }

    pub fn spawn(&self, mut command: Command, greeting: &str) -> Process {
        let sequence = self.sequence.get();
        self.sequence.set(sequence + 1);
        let log = self.root.path().join(format!("process-{sequence}.log"));
        let output = File::create(&log).unwrap();
        let child = command
            .env(
                "OEA_TEST_CONTROL_ADDR",
                self.listener.local_addr().unwrap().to_string(),
            )
            .stdin(Stdio::null())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
        let mut process = Process {
            child,
            peer: None,
            log: log.clone(),
        };
        process.peer = Some(self.accept(log));
        process.expect(greeting);
        process
    }

    fn accept(&self, log: PathBuf) -> Peer {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                    stream.set_write_timeout(Some(TIMEOUT)).unwrap();
                    return Peer {
                        stream: BufReader::new(stream),
                        helper: false,
                        closed: false,
                        log,
                    };
                }
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!(
                    "waiting for test host: {error}\n{}",
                    fs::read_to_string(&log).unwrap_or_default()
                ),
            }
        }
    }

    pub fn helper(&self) -> Peer {
        let mut peer = self.accept(
            self.root
                .path()
                .join(format!("process-{}.log", self.sequence.get() - 1)),
        );
        peer.helper = true;
        peer.expect("helper:v1");
        peer
    }

    pub fn assert_updated(&self) {
        assert_eq!(
            fs::read(self.root.path().join(APP)).unwrap(),
            fs::read(env!("CARGO_BIN_EXE_update-host-v2")).unwrap()
        );
        self.assert_resources_updated();
    }

    pub fn assert_resources_updated(&self) {
        assert_eq!(
            fs::read_to_string(self.root.path().join("resources/new.txt")).unwrap(),
            "new resource"
        );
        assert!(!self.root.path().join("resources/old.txt").exists());
        assert_eq!(
            fs::read_to_string(self.root.path().join("settings.json")).unwrap(),
            "user settings"
        );
        assert!(!self.root.path().join("cache/downloads/update.zip").exists());
        assert!(
            !self
                .root
                .path()
                .join("cache/update/transaction.json")
                .exists()
        );
        assert!(!self.root.path().join("cache/update/candidate").exists());
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        // 断言可能在驱动接收 helper 连接前失败。终止仍排队等待 run 的副本。
        while let Ok((mut stream, _)) = self.listener.accept() {
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let _ = writeln!(stream, "abort");
            let _ = io::copy(&mut stream, &mut io::sink());
        }
    }
}

pub struct Peer {
    stream: BufReader<TcpStream>,
    helper: bool,
    closed: bool,
    log: PathBuf,
}
impl Peer {
    pub fn send(&mut self, value: &str) {
        writeln!(self.stream.get_mut(), "{value}").unwrap();
    }
    pub fn next(&mut self) -> String {
        let mut line = String::new();
        let count = self.stream.read_line(&mut line).unwrap_or_else(|error| {
            panic!(
                "host event: {error}\n{}",
                fs::read_to_string(&self.log).unwrap_or_default()
            )
        });
        assert_ne!(
            count,
            0,
            "host disconnected\n{}",
            fs::read_to_string(&self.log).unwrap_or_default()
        );
        line.trim_end().to_owned()
    }
    pub fn expect(&mut self, expected: &str) {
        assert_eq!(
            self.next(),
            expected,
            "{}",
            fs::read_to_string(&self.log).unwrap_or_default()
        );
    }
    #[cfg(windows)]
    pub fn assert_waiting(&mut self) {
        self.stream
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(350)))
            .unwrap();
        let mut line = String::new();
        let error = self
            .stream
            .read_line(&mut line)
            .expect_err("helper completed while old executable was running");
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        self.stream
            .get_mut()
            .set_read_timeout(Some(TIMEOUT))
            .unwrap();
    }
    pub fn closed(&mut self) {
        let mut line = String::new();
        assert_eq!(
            self.stream.read_line(&mut line).unwrap(),
            0,
            "unexpected event: {line}"
        );
        self.closed = true;
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if self.helper && !self.closed {
            // helper 的控制线程在控制连接断开或收到 abort 时终止测试进程。
            let _ = writeln!(self.stream.get_mut(), "abort");
            let mut line = String::new();
            while matches!(self.stream.read_line(&mut line), Ok(count) if count > 0) {
                line.clear();
            }
        }
    }
}

pub struct Process {
    child: Child,
    peer: Option<Peer>,
    log: PathBuf,
}
impl Process {
    pub fn next(&mut self) -> String {
        self.peer.as_mut().unwrap().next()
    }
    #[cfg(windows)]
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    pub fn expect(&mut self, expected: &str) {
        self.peer.as_mut().unwrap().expect(expected);
    }
    pub fn send(&mut self, value: &str) {
        self.peer.as_mut().unwrap().send(value);
    }
    pub fn wait(&mut self, success: bool) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(
                    status.success(),
                    success,
                    "{}",
                    fs::read_to_string(&self.log).unwrap()
                );
                return;
            }
            assert!(
                Instant::now() < deadline,
                "host did not exit: {}",
                fs::read_to_string(&self.log).unwrap()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
