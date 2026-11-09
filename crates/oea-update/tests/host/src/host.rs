//! 通过公开更新 Interface 运行的测试宿主；控制连接只用于测试驱动与进程协调。
use oea_update::{
    Updater,
    install::{self, HelperHost, HelperLaunch, HelperLocale, InstallTarget},
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    net::TcpStream,
    path::Path,
    thread,
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn send(stream: &mut TcpStream, event: &str) -> io::Result<()> {
    writeln!(stream, "{event}")
}

fn command(stream: &mut TcpStream) -> io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            return String::from_utf8(bytes).map_err(io::Error::other);
        }
        bytes.push(byte[0]);
    }
}

pub fn main(version: u8) {
    if let Err(error) = run(version) {
        eprintln!("test host: {error}");
        std::process::exit(1);
    }
}

fn run(version: u8) -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().ok_or("missing test host mode")?;
    let mut control = TcpStream::connect(std::env::var("OEA_TEST_CONTROL_ADDR")?)?;
    control.set_read_timeout(Some(Duration::from_secs(15)))?;
    control.set_write_timeout(Some(Duration::from_secs(15)))?;
    if mode == "helper-entry" {
        send(&mut control, &format!("helper:v{version}"))?;
        if command(&mut control)? != "run" {
            return Err("helper was aborted".into());
        }
        // 驱动失败或断开时终止副本，避免遗留重试中的进程或占用临时目录。
        control.set_read_timeout(None)?;
        let mut watchdog = control.try_clone()?;
        thread::spawn(move || {
            let _ = command(&mut watchdog);
            std::process::exit(2);
        });
        install::run_helper(args, HelperLocale::ZhCn, Host(control));
    }
    let actual = std::env::current_exe()?;
    let root = actual.parent().ok_or("host has no parent directory")?;
    let name = actual.file_name().ok_or("host has no executable name")?;
    let target = InstallTarget::new(root, name)?;
    if mode == "start" {
        send(&mut control, &format!("start:v{version}"))?;
        let result = install::complete_startup_transaction(&target, || {
            send(&mut control, "resources").unwrap();
        });
        match result {
            Ok(result) => send(&mut control, &format!("startup:{result:?}"))?,
            Err(error) => {
                send(&mut control, "startup-error")?;
                return Err(error.into());
            }
        }
    } else if mode == "install" {
        send(&mut control, &format!("install:v{version}"))?;
        let package = args.next().ok_or("missing package path")?;
        let mode = args.next().unwrap_or_default();
        let updater = Updater::default();
        let result = updater.install_local(
            || {
                if mode == "cancel" {
                    let error = updater
                        .install_local(
                            || Ok(None),
                            &target,
                            || panic!("busy install must not start helper"),
                            |_| {},
                        )
                        .unwrap_err();
                    assert_eq!(error.reason, oea_update::Reason::Busy);
                    send(&mut control, "busy").unwrap();
                    Ok(None)
                } else {
                    Ok(Some(package.into()))
                }
            },
            &target,
            || {
                let source = if mode == "copy-failure" {
                    root.join("missing-helper")
                } else if mode == "spawn-failure" {
                    let source = root.join("invalid-helper");
                    fs::write(&source, "not an executable").map_err(|error| error.to_string())?;
                    source
                } else {
                    actual.clone()
                };
                Ok(HelperLaunch::new(
                    source,
                    [OsString::from("helper-entry")],
                    &target,
                    HelperLocale::EnUs,
                ))
            },
            |_| {},
        );
        match result {
            Ok(Some(_)) => {
                assert!(updater.is_installing());
                send(&mut control, "installed")?;
                if command(&mut control)? != "exit" {
                    return Err("unexpected install command".into());
                }
            }
            Ok(None) => {
                assert!(!updater.is_installing());
                send(&mut control, "cancelled")?;
            }
            Err(error) => {
                assert!(!updater.is_installing());
                send(&mut control, &format!("error:{:?}", error.reason))?;
                return Err(error.to_string().into());
            }
        }
    } else {
        return Err("unknown test host mode".into());
    }
    Ok(())
}

struct Host(TcpStream);
impl Drop for Host {
    fn drop(&mut self) {
        let _ = send(&mut self.0, "host-dropped");
    }
}
struct LogGuard(TcpStream);
impl Drop for LogGuard {
    fn drop(&mut self) {
        let _ = send(&mut self.0, "log-dropped");
    }
}
struct Prompt(TcpStream);
impl Drop for Prompt {
    fn drop(&mut self) {
        let _ = send(&mut self.0, "prompt-dropped");
    }
}
impl HelperHost for Host {
    type LogGuard = LogGuard;
    type Prompt = Prompt;
    fn init_logging(&mut self, root: &Path) -> LogGuard {
        fs::create_dir_all(root.join("logs")).unwrap();
        send(&mut self.0, "logging").unwrap();
        LogGuard(self.0.try_clone().unwrap())
    }
    fn show_progress(&mut self, locale: HelperLocale) -> Prompt {
        send(&mut self.0, &format!("progress:{locale:?}")).unwrap();
        Prompt(self.0.try_clone().unwrap())
    }
    fn show_success(&mut self, prompt: Prompt, locale: HelperLocale) {
        send(&mut self.0, &format!("success:{locale:?}")).unwrap();
        drop(prompt);
    }
    fn show_error(&mut self, locale: HelperLocale, _: &str) {
        send(&mut self.0, &format!("error:{locale:?}")).unwrap();
    }
}
