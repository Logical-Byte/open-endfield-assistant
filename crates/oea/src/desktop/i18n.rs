//! 原生桌面的文案。游戏标题与 OCR 观察不经过界面语言转换。
use super::*;
use crate::locale::UiLocale;

pub(super) struct Language(pub UiLocale);
impl Global for Language {}

pub(super) fn label(text: impl AsRef<str>, cx: &App) -> SharedString {
    let text = text.as_ref();
    if !cx
        .try_global::<Language>()
        .is_some_and(|language| language.0 == UiLocale::EnUs)
    {
        return text.to_owned().into();
    }
    let translated = match text {
        "档案扫描" => "Archive Scan",
        "百趣演武" => "Arena of Games",
        "日志" => "Logs",
        "监控" => "Monitor",
        "设置" => "Settings",
        "帮助" => "Help",
        "档案目录" => "Archive catalog",
        "扫描记录" => "Scan records",
        "命中记录" => "Matched hands",
        "未收集" => "Not collected",
        "已收集" => "Collected",
        "全部" => "All",
        "待确认" => "Needs review",
        "已匹配" => "Matched",
        "全部分类" => "All categories",
        "地图拾取" => "Map pickups",
        "导出到地图集" => "Export to OEM",
        "复制收集结果" => "Copy collection",
        "开始扫描" => "Start scan",
        "停止扫描" => "Stop scan",
        "清空" => "Clear",
        "撤销" => "Undo",
        "收起" => "Collapse",
        "编辑匹配" => "Edit assignment",
        "清除匹配" => "Clear assignment",
        "人工清除" => "Manually cleared",
        "人工匹配" => "Manual assignment",
        "精确匹配" => "Exact match",
        "自动采纳" => "Automatically adopted",
        "原始 OCR 识别结果" => "Original OCR text",
        "正文首行" => "Content preview",
        "档案分类" => "Category",
        "获取方式" => "Acquisition",
        "收集状态" => "Collection status",
        "纸质记录" => "Manuscripts",
        "电子档案" => "Electronic Records",
        "藏品" => "Collectibles",
        "多媒体" => "Multimedia",
        "中枢档案" => "Nexus Files",
        "调查报告" => "Investigative Reports",
        "关闭预览" => "Close preview",
        "没有符合条件的档案" => "No archives match your filters",
        "没有符合条件的扫描记录" => "No scan records match your filters",
        "点击 “开始扫描” 开始体验" => "Start a scan to collect records",
        "停止条件" => "Stop conditions",
        "同花顺" => "Straight Flush",
        "四条" => "Quad",
        "同花" => "Flush",
        "葫芦" => "Full House",
        "顺子" => "Straight",
        "三条" => "Triplet",
        "两对" => "Two Pair",
        "对子" => "Pair",
        "开始" => "Start",
        "停止" => "Stop",
        "暂无命中记录" => "No matched hands yet",
        "允许不足五张时命中停止条件" => {
            "Allow matches before all five cards are drawn"
        }
        "留空表示不启用该条件" => "Leave a field empty to disable that condition",
        "打开日志目录" => "Open log directory",
        "清空日志" => "Clear logs",
        "全部等级" => "All levels",
        "开始监控" => "Start monitoring",
        "停止监控" => "Stop monitoring",
        "等待游戏窗口截图" => "Waiting for a game screenshot",
        "语言与外观" => "Language and appearance",
        "简体中文" => "Simplified Chinese",
        "浅色" => "Light",
        "深色" => "Dark",
        "应用行为" => "Application",
        "关闭窗口时最小化到托盘" => "Minimize to tray on close",
        "自动下载更新" => "Download updates automatically",
        "自动安装更新" => "Install updates automatically",
        "音效音量（0–100）" => "Sound volume (0–100)",
        "更新" => "Updates",
        "检查更新" => "Check for updates",
        "下载" => "Download",
        "取消下载" => "Cancel download",
        "安装" => "Install",
        "开发者选项" => "Developer options",
        "执行方式：模拟" => "Worker: simulation",
        "执行方式：真实游戏" => "Worker: game automation",
        "以管理员权限重启" => "Restart as administrator",
        "安装本地更新包" => "Install local update package",
        "关于 OEA" => "About OEA",
        "GPUI 原生桌面" => "Native GPUI desktop",
        "保存设置" => "Save settings",
        "保存中…" => "Saving…",
        "查看扫描指引" => "View scan guide",
        "档案扫描前的准备" => "Prepare for an archive scan",
        "确认并开始扫描" => "Confirm and start scanning",
        "返回" => "Back",
        "重试初始化" => "Retry initialization",
        "正在加载 OCR 模型与档案目录…" => "Loading OCR models and archive catalog…",
        "关闭" => "Close",
        "切换明暗主题" => "Toggle color mode",
        "展开或收起侧栏" => "Expand or collapse sidebar",
        "未构成牌型" => "No hand",
        "显示" => "Showing",
        "条扫描记录" => "scan records",
        "个档案" => "archives",
        "今日好运组合" => "Today's lucky hand",
        "更新源" => "Update source",
        "代理" => "Proxy",
        "不使用代理" => "No proxy",
        "系统代理" => "System proxy",
        "自定义代理" => "Custom proxy",
        "已保存" => "Saved",
        "已就绪" => "Ready",
        "正在运行" => "Running",
        "正在停止" => "Stopping",
        "已完成" => "Completed",
        "已匹配：" => "Assigned: ",
        "请先启动终末地并进入游戏。扫描会控制游戏窗口，遍历档案库并识别收集状态。" => {
            "Start Endfield and enter the game. The scan controls the game window to browse the archive and identify collected entries."
        }
        "扫描期间请不要操作游戏，保持窗口可见并关闭 HDR。游戏界面需使用简体中文。" => {
            "Keep the game visible, disable HDR, and avoid interacting during a scan. The game UI must use Simplified Chinese."
        }
        "按单引号开始或停止档案扫描。百趣演武使用分号热键。Alt+Delete 退出应用。" => {
            "Use the quote key to toggle scanning, semicolon to toggle Arena of Games, and Alt+Delete to quit."
        }
        "人工编辑匹配会保留原始识别文本与后端匹配建议。导出前请检查待确认记录。" => {
            "Manual assignments preserve the original OCR text and matching suggestions. Review unmatched records before exporting."
        }
        "档案扫描会连接真实游戏窗口，识别档案并生成扫描记录。点击记录可以预览截图，编辑匹配时仅选择同分类档案。" => {
            "Archive scans connect to the game and create OCR records. Preview screenshots and assign archives within the same category."
        }
        "百趣演武根据牌型、战力点之和、分数或战力点子串停止。条件满足任意一项即可停止，好运组合由后端按北京时间凌晨 4 点更新。" => {
            "Arena of Games stops when any selected hand, BP total, score, or BP pattern matches. The lucky hand changes at 04:00 Beijing time."
        }
        "单引号：开始或停止档案扫描。分号：使用当前条件开始或停止百趣演武。Alt+Delete：退出。任务热键仅在 OEA 或游戏窗口位于前台时生效。" => {
            "Quote toggles scanning. Semicolon toggles Arena of Games with the current conditions. Alt+Delete quits. Task shortcuts require OEA or the game to be in the foreground."
        }
        "自动化与游戏监控支持 Windows。macOS 可开发原生界面与运行模拟工作者。" => {
            "Game automation and monitoring support Windows. Native UI development and simulation workers also run on macOS."
        }
        _ => return text.to_owned().into(),
    };
    translated.into()
}
