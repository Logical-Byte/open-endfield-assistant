use std::io;
use std::path::Path;

use tracing::level_filters::LevelFilter;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::{
    Layer, filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

/// 初始化日志系统。
///
/// - 控制台输出 INFO 及以上级别（带颜色）
/// - 文件输出所有级别（TRACE 及以上），按天轮转到 `<logs_dir>/YYYY-mm-dd.log`
/// - `ort` 及 `onnxruntime` 模块的日志仅输出 WARN 及以上（屏蔽 ONNX Runtime 的冗余信息）
///
/// # 参数
/// - `logs_dir`: 日志输出目录（如 [`crate::app_paths::AppPaths::logs_dir`]）
///
/// 返回的 `WorkerGuard` 必须被持有，否则文件写入线程会在 drop 时被关闭。
/// 通常放在 `main()` 中 `let _guard = init(&paths.logs_dir);` 即可。
pub fn init(logs_dir: &Path) -> WorkerGuard {
    // 无需手动创建 logs 目录，tracing_appender 会自动创建
    // let _ = fs::create_dir_all(logs_dir);

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

    // 文件过滤器：默认 TRACE，ort 只记录 WARN 及以上
    let file_filter = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("ort", LevelFilter::WARN);

    // 输出到文件：去掉颜色（文件中 ANSI 转义序列无意义）
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
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
