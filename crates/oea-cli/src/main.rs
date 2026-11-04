//! Developer commands with filesystem paths and explicit image coordinates.
mod args;
mod commands;
mod ocr;
mod screenshot;
use clap::Parser;
use serde_json::json;
use std::{
    io::{self, Write},
    process::ExitCode,
    sync::{Arc, Mutex},
};
use tracing_subscriber::{Layer, layer::SubscriberExt};

fn main() -> ExitCode {
    match args::Cli::try_parse_from(std::env::args_os()) {
        Ok(cli) => {
            // Buffer production diagnostics so JSON failures can include them
            // inside the single error object without contaminating stdout.
            let diagnostics = Diagnostics::default();
            let writer = diagnostics.clone();
            let layer = tracing_subscriber::fmt::layer()
                .with_writer(move || writer.clone())
                .with_ansi(false)
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    *metadata.level() <= tracing::Level::WARN
                        || (*metadata.level() == tracing::Level::DEBUG
                            && metadata.fields().field("error").is_some())
                }));
            let subscriber = tracing_subscriber::registry().with(layer);
            tracing::subscriber::with_default(subscriber, || {
                let result = commands::execute(&cli.command);
                write_result(
                    &cli,
                    result,
                    &diagnostics.0.lock().unwrap(),
                    &mut io::stdout().lock(),
                    &mut io::stderr().lock(),
                )
            })
        }
        Err(error) => {
            let code = error.exit_code() as u8;
            let _ = error.print();
            ExitCode::from(code)
        }
    }
}

// Buffer warnings and error diagnostics until the result is known, so a JSON failure can
// include them inside its error object without contaminating the stderr stream.
#[derive(Clone, Default)]
struct Diagnostics(Arc<Mutex<Vec<u8>>>);

impl Write for Diagnostics {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_result(
    cli: &args::Cli,
    result: anyhow::Result<commands::Output, commands::Error>,
    diagnostics: &[u8],
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> ExitCode {
    match result {
        Ok(result) => {
            let _ = stderr.write_all(diagnostics);
            let written = if cli.json {
                writeln!(stdout, "{}", result.json)
            } else {
                writeln!(stdout, "{}", result.human)
            };
            if written.is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            if cli.json {
                let mut message = format!("{:#}", error.source);
                if !diagnostics.is_empty() {
                    message.push_str("\nDiagnostics:\n");
                    message.push_str(&String::from_utf8_lossy(diagnostics));
                }
                let _ = writeln!(
                    stderr,
                    "{}",
                    json!({"ok":false, "command":cli.command.name(),
                    "error":{"kind":error.kind,"message":message}})
                );
            } else {
                let _ = stderr.write_all(diagnostics);
                let _ = writeln!(stderr, "{}: {:#}", cli.command.name(), error.source);
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_runtime_error_keeps_diagnostics_inside_the_single_error_object() {
        let cli = args::Cli::try_parse_from(["oea-cli", "connect", "--json"]).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = write_result(
            &cli,
            Err(commands::Error {
                kind: "runtime_error",
                source: anyhow::anyhow!("connection failed"),
            }),
            "WARN 检查显示器 HDR 状态失败\n".as_bytes(),
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(code, ExitCode::FAILURE);
        assert!(stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["error"]["kind"], "runtime_error");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("connection failed")
        );
        assert!(error["error"]["message"].as_str().unwrap().contains("HDR"));
    }
}
