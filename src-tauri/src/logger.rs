use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;

use tracing::level_filters::LevelFilter;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::{
    Layer, filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

/// 前端日志写入器：把格式化后的日志按行发送到指定通道，供 Tauri 转发到界面。
#[derive(Clone)]
struct ChannelWriter {
    tx: mpsc::Sender<String>,
    buf: Vec<u8>,
}

impl Write for ChannelWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        // 每遇到换行就发送完整的一行（去掉行尾换行/回车）
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line).trim_end().to_string();
            let _ = self.tx.send(line);
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::MakeWriter<'a> for ChannelWriter {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter {
            tx: self.tx.clone(),
            buf: Vec::new(),
        }
    }
}

/// 初始化日志系统。
///
/// - 控制台输出 INFO 及以上级别（带颜色）
/// - 文件输出所有级别（TRACE 及以上），按天轮转到 `<logs_dir>/YYYY-mm-dd.log`
/// - 前端转发所有级别（TRACE 及以上），逐行发送到返回的接收端（供 Tauri 界面实时显示）
/// - `ort` 及 `onnxruntime` 模块的日志仅输出 WARN 及以上（屏蔽 ONNX Runtime 的冗余信息）
///
/// # 参数
/// - `logs_dir`: 日志输出目录（如 [`crate::app_paths::AppPaths::logs_dir()`]）
///
/// 返回 `(WorkerGuard, mpsc::Receiver<String>)`：
/// - `WorkerGuard` 必须被持有，否则文件写入线程会在 drop 时被关闭；
/// - `Receiver` 每收到一行日志，即由上层转发给 Tauri 前端。
pub fn init(logs_dir: &Path) -> (WorkerGuard, mpsc::Receiver<String>) {
    // 前端转发通道
    let (tx, rx) = mpsc::channel::<String>();

    // 按天轮转的文件写入器，输出到 logs_dir/YYYY-mm-dd.log
    let file_appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("")
        .filename_suffix("log")
        .build(logs_dir)
        .expect("初始化文件日志失败");

    // 非阻塞文件写入（独立线程）
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 控制台过滤器：默认 INFO，ort 只记录 WARN 及以上
    let console_filter = Targets::new()
        .with_default(LevelFilter::INFO)
        .with_target("ort", LevelFilter::WARN);

    // 输出到 stderr：INFO 及以上（带颜色）
    let console_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_filter(console_filter);

    // 文件/前端过滤器：默认 TRACE，ort 只记录 WARN 及以上
    let frontend_filter = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("ort", LevelFilter::WARN);

    // 输出到文件：去掉颜色（文件中 ANSI 转义序列无意义）
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_filter(frontend_filter.clone());

    // 输出到前端通道：去掉颜色，逐行发送
    let frontend_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(ChannelWriter {
            tx,
            buf: Vec::new(),
        })
        .with_filter(frontend_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(frontend_layer)
        .init();

    (guard, rx)
}

/// 记录一条 SUCCESS 级别的日志（基于 tracing INFO 级别，但标记为 SUCCESS）。
///
/// 用于输出扫描到的档案标题等关键业务结果。
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
        tracing::info!("SUCCESS: {}", format!($($arg)*))
    };
}
