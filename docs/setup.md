# 环境配置与本地调试

## 环境要求

- Node.js：建议使用 22 LTS。
- pnpm：版本见 [package.json](../package.json) 的 `packageManager`，建议启用 Corepack。
- Rust：使用 stable，最低版本见 [Cargo.toml](../src-tauri/Cargo.toml) 的 `rust-version`。
- Windows 10 / 11（x86_64）：后端编译需要 MSVC（Microsoft C++ Build Tools，含 Windows SDK）。平台支持范围见[后端规范](rule-backend.md#编译)。

WebView2 缺失时，应用会在首次启动时引导下载安装。

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

模型位于 `resources/ocr-models/`，不纳入 Git。Tauri 开发与构建命令会自动下载缺失模型，配置见 [tauri.conf.json](../src-tauri/tauri.conf.json)。也可手动下载：

```bash
pnpm download:models
```

已有文件会跳过；强制重新下载使用 `pnpm download:models --force`。模型来源见[第三方组件](third-party-notices.md)。

## 本地调试

以下命令在仓库根目录运行。

- 完整应用（Windows）：`pnpm tauri dev`，同时启动 Vite 开发服务器与 Tauri 窗口。
- 仅前端：`pnpm dev`，无法调用后端命令（`invoke` 会失败），适合纯 UI 调试。

macOS 开发外壳是可选的本地调试入口。初始化 `resources/` 子模块后，可直接运行 `pnpm tauri dev`。能力边界见[后端规范](rule-backend.md#编译)。

## 后端日志

- 开发期控制台输出 DEBUG 及以上级别。
- 文件日志位于应用根目录下的 `logs/YYYY-mm-dd.log`，记录 TRACE 及以上级别，按本地日期每日轮换；开发期的应用根目录为项目根目录。
