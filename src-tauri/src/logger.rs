use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

use chrono::{Local, NaiveDate};
use serde::Serialize;
use tracing::{Event, Subscriber, level_filters::LevelFilter};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_log::LogTracer;
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
    /// 时间（本地时间，ISO 8601 字符串，含微秒与时区偏移，如 `2026-08-06T12:34:56.123456+08:00`）
    pub time: String,
    /// 日志等级：TRACE / DEBUG / INFO / WARN / ERROR
    pub level: String,
    /// 格式化后的日志文本（事件字段，不含时间 / 等级 / 调用者）
    pub message: String,
}

/// 按本地日期每日轮换的文件写入器（tracing-appender 本身仅按 UTC 轮换，故自定义）。
struct DailyRotatingWriter {
    inner: Arc<Mutex<DailyState>>,
}

/// 每日轮换状态：当前打开的日志文件与所属本地日期。
struct DailyState {
    dir: PathBuf,
    current_date: Option<NaiveDate>,
    file: Option<File>,
}

impl DailyRotatingWriter {
    fn new(dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DailyState {
                dir,
                current_date: None,
                file: None,
            })),
        }
    }

    /// 确保 `date` 对应的日志文件已打开；日期变化时切换到新文件。
    fn ensure_file(state: &mut DailyState, date: NaiveDate) -> io::Result<()> {
        if state.current_date == Some(date) {
            return Ok(());
        }
        fs::create_dir_all(&state.dir)?;
        let path = state.dir.join(format!("{}.log", date.format("%Y-%m-%d")));
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        state.file = Some(file);
        state.current_date = Some(date);
        Ok(())
    }
}

impl Write for DailyRotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let date = Local::now().date_naive();
        let mut state = self.inner.lock().unwrap();
        Self::ensure_file(&mut state, date)?;
        state.file.as_mut().expect("日志文件已打开").write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.inner.lock().unwrap();
        if let Some(file) = state.file.as_mut() {
            file.flush()?;
        }
        Ok(())
    }
}

/// 前端日志写入层：把每个事件格式化为 `LogEntry` 并通过通道发送。
struct ChannelLayer {
    tx: mpsc::Sender<LogEntry>,
    /// 时间格式器（本地时间 ISO 8601，含微秒与时区偏移，精度与文件日志一致）
    timer: ChronoLocal,
}

impl ChannelLayer {
    fn new(tx: mpsc::Sender<LogEntry>) -> Self {
        Self {
            tx,
            timer: ChronoLocal::new("%Y-%m-%dT%H:%M:%S%.6f%:z".to_string()),
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
/// - 控制台输出 DEBUG 及以上级别（带颜色），显示 `MM-dd HH:MM:SS`、等级、调用者、行号与信息
/// - 文件输出所有级别（TRACE 及以上），信息最完整：本地时间（含时区偏移）、等级、调用者、
///   源文件位置、行号、线程名 / ID 与全部字段，按**本地日期**轮转到 `<logs_dir>/YYYY-mm-dd.log`
/// - 前端转发所有级别（TRACE 及以上），以 `LogEntry`（时间 + 等级 + 文本）逐条发送，
///   时间为本地 ISO 8601 字符串（含微秒与时区偏移），由前端自行格式化展示，并可按等级过滤
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

    // 按本地日期每日轮换的文件写入器，输出到 logs_dir/YYYY-mm-dd.log
    let file_writer = DailyRotatingWriter::new(logs_dir.to_path_buf());

    // 非阻塞文件写入（独立线程）
    let (non_blocking, guard) = tracing_appender::non_blocking(file_writer);

    // 控制台过滤器：默认 DEBUG，其他模块只记录 WARN 及以上
    let console_filter = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target("ort", LevelFilter::WARN)
        .with_target("tao", LevelFilter::WARN);

    // 输出到 stderr：DEBUG 及以上（带颜色），格式为 `MM-dd HH:MM:SS 等级 调用者: 行号: 信息`
    let console_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_timer(ChronoLocal::new("%m-%d %H:%M:%S".to_string()))
        .with_line_number(true)
        .with_filter(console_filter);

    // 文件/前端过滤器：默认 TRACE，其他模块只记录 WARN 及以上
    let frontend_filter = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("ort", LevelFilter::WARN)
        .with_target("tao", LevelFilter::WARN);

    // 输出到文件：本地时间（含时区偏移）、等级、调用者、源文件位置、行号、线程与字段
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.6f%:z".to_string()))
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_writer(non_blocking)
        .with_filter(frontend_filter.clone());

    // 输出到前端通道：结构化 LogEntry（等级 + 文本），供界面按等级过滤
    let frontend_layer = ChannelLayer::new(tx).with_filter(frontend_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(frontend_layer)
        .init();

    // 桥接 `log` crate → tracing：Tauri 及插件内部使用 `log` crate 记录消息
    // （如 setup 失败时的 `Failed to setup app`），不桥接的话这些消息会丢失，
    // 不会进入日志文件与前端。重复安装会返回 Err，忽略即可。
    let _ = LogTracer::init();

    (guard, rx)
}
