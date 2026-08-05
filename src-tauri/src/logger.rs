use std::io;
use std::path::Path;
use std::sync::mpsc;

use serde::Serialize;
use tracing::{Event, Subscriber, level_filters::LevelFilter};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::{
    Layer,
    filter::Targets,
    fmt,
    fmt::format::{DefaultFields, FormatFields, Writer},
    fmt::time::{ChronoLocal, FormatTime},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

/// 推送给前端的日志条目（时间 + 等级 + 格式化文本）。
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// 时间（本地时间，`MM-dd HH:MM:SS`）
    pub time: String,
    /// 日志等级：TRACE / DEBUG / INFO / WARN / ERROR
    pub level: String,
    /// 格式化后的日志文本（事件字段，不含时间 / 等级 / 调用者）
    pub message: String,
}

/// 前端日志写入层：把每个事件格式化为 `LogEntry` 并通过通道发送。
struct ChannelLayer {
    tx: mpsc::Sender<LogEntry>,
    /// 时间格式器（与控制台共用同一 `MM-dd HH:MM:SS` 本地时间格式）
    timer: ChronoLocal,
}

impl ChannelLayer {
    fn new(tx: mpsc::Sender<LogEntry>) -> Self {
        Self {
            tx,
            timer: ChronoLocal::new("%m-%d %H:%M:%S".to_string()),
        }
    }
}

impl<S> Layer<S> for ChannelLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // 只格式化事件字段（message 及命名字段），等级单独携带供界面过滤
        let mut message = String::new();
        let writer = Writer::new(&mut message);
        let _ = DefaultFields::default().format_fields(writer, event);

        // 时间由 FormatTime 在事件到达时实时取系统时间生成（tracing 事件本身不携带时间戳）
        let mut time = String::new();
        let mut writer = Writer::new(&mut time);
        let _ = self.timer.format_time(&mut writer);

        let _ = self.tx.send(LogEntry {
            time,
            level: event.metadata().level().to_string(),
            message,
        });
    }
}

/// 初始化日志系统。
///
/// - 控制台输出 DEBUG 及以上级别（带颜色），显示 `MM-dd HH:MM:SS`、等级、调用者与信息
/// - 文件输出所有级别（TRACE 及以上），信息最完整（完整时间戳、等级、调用者、字段），
///   按天轮转到 `<logs_dir>/YYYY-mm-dd.log`
/// - 前端转发所有级别（TRACE 及以上），以 `LogEntry`（时间 + 等级 + 文本）逐条发送，
///   供 Tauri 界面展示 `MM-dd HH:MM:SS`、等级与信息，并可按等级过滤
/// - `ort` 及 `onnxruntime` 模块的日志仅输出 WARN 及以上（屏蔽 ONNX Runtime 的冗余信息）
///
/// # 参数
/// - `logs_dir`: 日志输出目录（如 [`crate::app_paths::AppPaths::logs_dir()`]）
///
/// 返回 `(WorkerGuard, mpsc::Receiver<LogEntry>)`：
/// - `WorkerGuard` 必须被持有，否则文件写入线程会在 drop 时被关闭；
/// - `Receiver` 每收到一条日志，即由上层转发给 Tauri 前端。
pub fn init(logs_dir: &Path) -> (WorkerGuard, mpsc::Receiver<LogEntry>) {
    // 前端转发通道
    let (tx, rx) = mpsc::channel();

    // 按天轮转的文件写入器，输出到 logs_dir/YYYY-mm-dd.log
    let file_appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("")
        .filename_suffix("log")
        .build(logs_dir)
        .expect("初始化文件日志失败");

    // 非阻塞文件写入（独立线程）
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 控制台过滤器：默认 DEBUG，ort 只记录 WARN 及以上
    let console_filter = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target("ort", LevelFilter::WARN);

    // 输出到 stderr：DEBUG 及以上（带颜色），格式为 `MM-dd HH:MM:SS 等级 调用者: 信息`
    let console_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_timer(ChronoLocal::new("%m-%d %H:%M:%S".to_string()))
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

    // 输出到前端通道：结构化 LogEntry（等级 + 文本），供界面按等级过滤
    let frontend_layer = ChannelLayer::new(tx).with_filter(frontend_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(frontend_layer)
        .init();

    (guard, rx)
}
