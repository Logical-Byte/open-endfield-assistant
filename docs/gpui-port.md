# GPUI 桌面迁移

默认入口 `crates/oea/src/main.rs` 启动 GPUI。自动化、OCR、导航、设置存储、日志与更新继续使用原有 Rust 实现。原生界面位于 `crates/oea/src/desktop/`，不经过 Tauri IPC。旧 Tauri 入口在 `legacy-tauri` feature 下保留，便于迁移期间回归比较。

## 开发与后台验证

在仓库根目录运行：

```bash
cargo build -p oea --bin oea
cargo test -p oea --lib
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

这些命令不会打开桌面窗口。手动体验时运行 `pnpm native`，它先准备 OCR 模型，再启动应用。也可在模型已经准备好的情况下直接运行 `cargo run -p oea --bin oea`。

原生开发构建从工作区读取资源并禁用自更新。`cargo build -p oea --bin oea --release` 生成的发行构建从可执行文件旁读取资源。Windows 便携打包使用 `pnpm package`，不需要 Vite 或 WebView 构建产物。版本号与产品名暂时继续读取 `tauri.conf.json` 中已有的元数据，以兼容现有发版和更新流程。

## 模块与行为

- `desktop/backend.rs` 装配真实 `Controller`、OCR、导航器和自动化运行时。领域事件通过内存通道进入 GPUI。日志与全局键盘监听由 `BackgroundThreads` 持有。
- `desktop/archive.rs` 消费后端 `ScannedItem`，保留原始观察和匹配建议。人工选择优先于自动采纳，人工清除不会回退到自动匹配。撤销按稳定扫描记录标识恢复人工决定。OEM 导出保留档案目录顺序与现有协议。
- `desktop/ui.rs` 展示档案目录、扫描记录、截图预览、搜索和筛选。`desktop/pages.rs` 提供百趣演武、监控、设置、日志与帮助页面。
- 百趣演武使用原有工作者、条件校验、每日好运组合和计分结果。开发者选项中的模拟方式也使用原有模拟工作者。
- 监控从真实游戏窗口截图，后台截图完成后发布原生图像。上一帧尚未完成时不会启动下一帧，离开监控页面后停止调度。
- 设置提交成功后才更换已生效快照。Mirror酱 CDK 使用原有平台数据保护加密。外观保存在独立的 `config/appearance.json`。
- 更新检查、下载、取消和安装流程已改为普通 Rust 函数。下载进度使用回调，旧 Tauri adapter 与 GPUI 共用同一状态机和事务实现。自动安装等待自动化空闲，安装期间拒绝退出。
- 引号、分号和 Alt+Delete 沿用原有 Windows 全局键盘监听。托盘提供打开、扫描启停和退出操作。

## 平台与验收范围

Windows x86_64 仍是真实游戏能力的目标平台。macOS 用于编译、测试和原生界面开发，无法验证 Windows 游戏窗口、UAC、全局热键和实际更新替换。

本次开发采用后台构建和测试，不自动打开窗口。界面截图对照、中文输入法、窗口尺寸变化与逐项点击验收，需要在用户主动启动应用后完成。当前实现应视为正在迁移的原生桌面入口，尚不能据此宣称所有界面行为与旧版完全一致。
