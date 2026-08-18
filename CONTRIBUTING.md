# Contributing Guide

欢迎参与 **Open Endfield Assistant（OEA）** 的开发。OEA 是一个活跃的社区项目，欢迎每一位大佬参与开发和维护。

本文档面向开发者，涵盖开发环境准备、参与方式与本地调试。

用户使用相关的内容见仓库根目录的 [README.md](README.md)。

## 项目概览

- **技术栈**：前端 Vue 3 + Nuxt UI（Vue 版）+ Vite + TypeScript；桌面壳 Tauri 2（Rust）；OCR 使用 RapidOCR（ONNX 模型，位于 `resources/ocr-models/`）。
- **仓库结构**：
  - `src/` — 前端（Nuxt UI Vue 版）
  - `src-tauri/` — Rust 后端（Tauri 2）
  - `resources/` — git submodule（`Logical-Byte/oea-resource`），前后端共享资产（模板图、游戏数据、图标）
  - `resources/ocr-models/` — OCR ONNX 模型（约 32MB，已 git 忽略）
  - `scripts/` — 数据生成（`makeAllData.ts`）、打包（`package.ts`）等脚本
  - `.github/workflows/ci.yml` — CI（前端 check + 后端 fmt/clippy/test）

## 开发环境准备

| 依赖    | 版本要求                                        | 说明                           |
| ------- | ----------------------------------------------- | ------------------------------ |
| Node.js | ≥ 20.19（建议 22 LTS）                          | Vite 7 要求                    |
| pnpm    | 11.x（仓库锁定 `packageManager: pnpm@11.17.0`） | 建议启用 corepack              |
| Rust    | ≥ 1.85（stable）                                | 仓库使用 `edition = "2024"`    |
| Windows | 10 / 11（x86_64）                               | 当前仅面向 Windows x86_64 分发 |

> WebView2 运行时为 Tauri 运行时依赖，缺失时应用会在首次启动时自动联网安装，无需手动准备。

后端编译需要 MSVC（Microsoft C++ Build Tools，含 Windows SDK）。建议使用 VS Code 并安装 `rust-analyzer`、`Volar`（Vue 官方扩展）。

### 获取代码

仓库使用 git submodule（`resources/`），克隆时请一并拉取：

```bash
git clone --recurse-submodules git@github.com:Logical-Byte/open-endfield-assistant.git
# 如果已经克隆但没有子模块：
git submodule update --init --recursive
```

### 安装依赖

```bash
pnpm install
```

### 下载 OCR 模型

`resources/ocr-models/` 目录已被 git 忽略（体积较大，不入库），克隆仓库后需要先下载 OCR 模型才能正常识别档案标题：

```bash
pnpm download:models
```

已存在的文件会跳过，需要强制覆盖时加 `--force`（`pnpm download:models --force`）。下载源为 ModelScope 的 `RapidAI/RapidOCR` 仓库，脚本位于 `scripts/downloadModels.ts`。

## 如何参与开发

1. 在 [GitHub](https://github.com/Logical-Byte/open-endfield-assistant) 上开 issue 说明要做什么，或认领已有 issue。
2. 从 `main` 分支切出功能分支：`git switch -c <your_name>/feat/xxx`，例如如果你的名字叫 `oceancat`，要做的功能是 `foo`，则分支名可为 `oceancat/feat/foo`。
3. 开发前阅读对应的代码规范：[前端代码规范](docs/rule-frontend.md) 或 [后端代码规范](docs/rule-backend.md)。
4. 完成改动后，本地通过全部检查（见下文「检查与测试」）。
5. 发起 Pull Request（PR），在描述中说明改动内容与验证情况。

- **提交信息**：建议遵循 Conventional Commits（`feat:` / `fix:` / `refactor:` / `docs:` 等），保持简洁清晰。

## 本地调试

### 启动完整应用（推荐）

```bash
pnpm tauri dev
```

同时启动 Vite 开发服务器（`http://localhost:1420`，热更新）与 Tauri 调试窗口。前端改动即时生效；Rust 改动会触发重新编译。

### 只启动前端

```bash
pnpm dev   # http://localhost:1420（strictPort）
```

此时无法调用后端命令（`invoke` 会失败），仅适合纯 UI 调试。

### 后端日志

- 开发期：控制台输出 DEBUG+ 级别；
- 文件日志：`logs/YYYY-mm-dd.log`（TRACE+），按本地日期每日轮换，位于应用根目录（开发期为项目根）。

### 检查与测试

前端改动运行：

```bash
pnpm fix
pnpm build
pnpm check        # typecheck + eslint + prettier 检查
```

后端命令在 `src-tauri/` 目录中运行：

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test  # 后端单元测试
```

### 数据生成

`resources/data/prts.json` 由解包数据生成，改了 `scripts/` 下的数据表脚本后需要重新生成：

```bash
pnpm makedata   # jiti scripts/makeAllData.ts
```

相关数据表说明见 [解包数据参考](docs/game-data.md)。

## 专题文档

- [前端代码规范](docs/rule-frontend.md)：前端依赖、TypeScript、Vue 组件和自动修复约定；
- [后端代码规范](docs/rule-backend.md)：Rust 依赖、代码写法和磁盘写入边界；
- [打包与发版](docs/releasing.md)：绿色便携包、冒烟测试和 GitHub Release 流程；
- [档案库自动化逻辑](docs/archive-automation.md)：界面特征、导航和扫描流程；
- [解包数据参考](docs/game-data.md)：档案相关数据表及标题差异。
