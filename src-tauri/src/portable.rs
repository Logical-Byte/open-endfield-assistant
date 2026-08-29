//! 便携运行环境检测：判断是否从压缩包 / 临时目录内直接运行。
//!
//! 背景：OEA 是绿色便携版，`cache/`、`logs/`、`config/` 都写在 exe 旁。用户从
//! 压缩包内直接双击 `OEA.exe` 时，压缩工具会把程序释放到临时目录（且可能只释放
//! 单个文件、目录只读），导致资源缺失或写入失败，启动报错且用户不知道要解压。
//!
//! 本模块在启动早期（建窗口 / 写盘之前）检测根目录，命中时弹原生框提示"请先解压"
//! 并退出。检测融合三路信号：
//! - 可写性探测（写探针文件）：覆盖只读临时目录（WinRAR 解压并运行等）；
//! - 关键词黑名单 + 系统临时目录前缀（借鉴 MXU 的 `check_exe_path` 思路）：覆盖
//!   路径特征命中（7-Zip 单文件释放、各压缩软件 / 下载器的临时目录等）；
//! - 资源缺失检查（`resources/ocr-models` 是否含模型）：提示解压不完整。

use std::fs;
use std::path::Path;

use crate::{app_paths::AppPaths, windows_ops};

/// 根目录运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    /// 正常：可写、不在临时目录、资源齐全。
    Ok,
    /// 疑似压缩包 / 临时目录内运行，附具体原因。
    ZipRuntime(ZipReason),
}

/// 判为压缩包 / 临时目录运行的具体原因（用于文案细分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipReason {
    /// 根目录不可写（WinRAR 解压并运行、Program Files 等只读位置）。
    ReadOnly,
    /// 位于临时目录且资源缺失（7-Zip 单文件释放等）。
    TempAndMissingResources,
    /// 位于临时目录但资源齐全、可写（资源管理器 zip 内运行等，数据会丢）。
    Temp,
}

/// 高置信的临时目录关键词（Windows 路径转小写后按子串匹配）。
///
/// 只收录明确到目录段的压缩软件 / 下载器 / 系统临时路径；不放 `\temp\`、`\tmp\`、
/// `\wz` 等宽泛词，避免误伤用户自建的 `D:\temp` 之类目录。
const TEMP_KEYWORDS: &[&str] = &[
    // 压缩软件临时目录
    "\\rar$",      // WinRAR
    "\\7zocab",    // 7-Zip
    "\\7zo",       // 7-Zip（短）
    "\\360zip",    // 360压缩
    "\\360xtract", // 360压缩
    "\\2345zip",   // 2345好压
    "\\haozip",    // 2345好压
    "\\kuaizip",   // 快压
    "\\bztmp",     // Bandizip
    "\\bandizip",  // Bandizip
    // 系统临时路径变体
    "\\appdata\\local\\temp",
    "\\temporary internet files",
    // 网盘 / 下载器
    "\\baiduyundownload",
    "\\baidupcs",
    "\\thundernetwork",
    "\\xunlei\\downloads\\.tmp",
    // 聊天软件
    "\\tencent\\qq\\temp",
    "\\tencent files",
    "\\weixin files",
];

/// 根目录是否可写：写入一个探针文件再删除，失败视为只读目录。
fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".oea-probe-{}", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 根目录是否落在「临时目录」：系统临时目录前缀，或命中高置信关键词。
fn is_temp_location(dir: &Path) -> bool {
    let normalized = dir.to_string_lossy().replace('/', "\\").to_lowercase();

    if normalized.starts_with(&temp_dir_normalized()) {
        return true;
    }
    TEMP_KEYWORDS.iter().any(|k| normalized.contains(k))
}

/// 系统临时目录（小写、`\` 分隔）作为前缀基准。
fn temp_dir_normalized() -> String {
    std::env::temp_dir()
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase()
}

/// 资源是否完整：`resources/ocr-models/` 目录存在且含任意 `.onnx` 模型文件。
fn resources_ok(root: &Path) -> bool {
    let models_dir = root.join("resources").join("ocr-models");
    if !models_dir.is_dir() {
        return false;
    }
    fs::read_dir(&models_dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| entry.path().extension().is_some_and(|ext| ext == "onnx"))
        })
        .unwrap_or(false)
}

/// 组合判定（纯函数，唯一判定入口）。
pub fn check(root: &Path) -> PathStatus {
    if !is_writable(root) {
        return PathStatus::ZipRuntime(ZipReason::ReadOnly);
    }
    if is_temp_location(root) {
        return if resources_ok(root) {
            PathStatus::ZipRuntime(ZipReason::Temp)
        } else {
            PathStatus::ZipRuntime(ZipReason::TempAndMissingResources)
        };
    }
    PathStatus::Ok
}

/// 启动早期调用：命中则写 crash 日志、弹原生框、退出；不命中则无副作用。
pub fn ensure_extracted(app_paths: &AppPaths) {
    let PathStatus::ZipRuntime(reason) = check(app_paths.root_dir()) else {
        return;
    };

    let _ = crate::crash::write_crash_log(
        "ZIP RUNTIME",
        &format!("{reason:?}, root = {}", app_paths.root_dir().display()),
    );

    let (title, content) = match reason {
        ZipReason::ReadOnly => (
            "请先解压 OEA 再运行",
            "OEA 是绿色便携版，必须在解压后的文件夹中运行。

当前程序所在的文件夹是只读的（通常因为你直接从压缩包内运行），无法写入配置、日志和缓存。

请按以下步骤操作：
1. 右键压缩包，选择「解压到当前文件夹」；
2. 打开解压出的文件夹；
3. 双击其中的 OEA.exe 启动。

解压后文件夹中应包含 OEA.exe 和 resources/ 文件夹。",
        ),
        ZipReason::TempAndMissingResources => (
            "请先解压 OEA 再运行",
            "OEA 是绿色便携版，必须在完整解压后运行。

检测到程序正在临时目录中运行，且资源文件缺失（压缩工具可能只释放了程序本身）。

请按以下步骤操作：
1. 右键压缩包，选择「解压到当前文件夹」；
2. 打开解压出的文件夹；
3. 双击其中的 OEA.exe 启动。

解压后文件夹中应包含 OEA.exe 和 resources/ 文件夹。",
        ),
        ZipReason::Temp => (
            "请在解压后的文件夹运行 OEA",
            "OEA 是绿色便携版，请在解压后的文件夹中运行。

检测到程序正在临时目录中运行，关闭后配置和日志可能丢失。

请按以下步骤操作：
1. 右键压缩包，选择「解压到当前文件夹」；
2. 打开解压出的文件夹；
3. 双击其中的 OEA.exe 启动。

解压后文件夹中应包含 OEA.exe 和 resources/ 文件夹。",
        ),
    };

    let _ =
        windows_ops::dialog::show_message(title, content, windows_ops::dialog::DialogIcon::Error);
    std::process::exit(1);
}
