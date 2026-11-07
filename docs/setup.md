# 环境配置与本地调试

## 环境要求

- Node.js：建议使用 22 LTS。
- pnpm：版本见 [package.json](../package.json) 的 `packageManager`，建议启用 Corepack。
- Rust：使用 stable，最低版本见 [Cargo.toml](../crates/oea/Cargo.toml) 的 `rust-version`。
- Windows 10 / 11（x86_64）：后端编译需要 MSVC（Microsoft C++ Build Tools，含 Windows SDK）。平台支持范围见[后端规范](rule-backend.md#编译检查与测试)。

默认桌面入口使用 GPUI，不需要 WebView2。旧 Tauri 入口保留在 `legacy-tauri` feature 中，供迁移期间回归对照。

## 准备代码与依赖

`resources/` 是前后端共享资产的 Git submodule，克隆时一并拉取：

```bash
git clone --recurse-submodules git@github.com:Logical-Byte/open-endfield-assistant.git
```

已有仓库缺少子模块时，在仓库根目录运行 `pnpm submodules:init`。

在仓库根目录安装依赖，同时安装 Lefthook Git hooks：

```bash
pnpm install
```

检查 hooks 配置与安装状态：

```bash
pnpm hooks:validate
pnpm exec lefthook check-install
pnpm hooks:pre-commit
```

## OCR 模型

模型位于 `resources/ocr-models/`，不纳入 Git。Tauri 开发与构建命令会自动下载缺失模型，配置见 [tauri.conf.json](../crates/oea/tauri.conf.json)。也可手动下载：

```bash
pnpm download:models
```

已有文件会跳过；强制重新下载使用 `pnpm download:models --force`。模型来源见[第三方组件](third-party-notices.md)。

## 本地调试

以下命令在仓库根目录运行。

- 原生完整应用（Windows）：`pnpm native`，准备 OCR 模型后启动 GPUI 窗口。
- 仅后台编译：`cargo build -p oea --bin oea`，不会启动窗口。
- 后台测试：`cargo test -p oea --lib`，不会启动桌面窗口或连接游戏。
- 旧 Tauri 入口：`pnpm tauri dev --features legacy-tauri -- --no-default-features`，供迁移期间回归对照。
- 仅前端：`pnpm dev`，无法调用后端命令（`invoke` 会失败），适合纯 UI 调试。

OEA dev 与 OEA release 的资源和更新行为差异见[构建形态](build-variants.md)。

macOS 可使用 `pnpm native` 开发原生界面与运行模拟工作者。GPUI 启用了运行时 Metal 着色器编译，Command Line Tools 即可编译。真实游戏自动化与监控仍只支持 Windows。能力边界见[后端规范](rule-backend.md#编译检查与测试)。

设置仍读写 `config/oea_config.json`。原生主题与侧栏状态保存在 `config/appearance.json`。扫描记录与人工匹配同原 Vue 前端一样只保留在本次会话中。原生入口说明见 [GPUI 迁移](gpui-port.md)。

调试档案扫描页面时，可在「设置 → 开发者选项」中勾选「使用模拟扫描结果」，然后点击档案扫描页面的「开始模拟扫描」。模拟扫描无需游戏窗口，约 20 秒内以随机间隔报告示例结果，支持停止和重新开始。该开关重启应用后关闭。

## 后端日志

不启动 Tauri 的导航、截图和模板匹配调试入口见[开发者 CLI](developer-cli.md)。

- 开发期控制台输出 DEBUG 及以上级别。
- 文件日志位于应用根目录下的 `logs/YYYY-mm-dd.log`，记录 TRACE 及以上级别，按本地日期每日轮换；开发期的应用根目录为项目根目录。
