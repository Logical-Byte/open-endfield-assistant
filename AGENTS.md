## 项目概览

Open Endfield Assistant（OEA）是一个 Tauri 2 桌面应用。前端使用 Vue 3、Nuxt UI、Vite 和 TypeScript，桌面后端使用 Rust。

## 前端

前端代码位于 `src/`，使用 pnpm 管理依赖。以下命令在仓库根目录运行：

```bash
pnpm install
pnpm tauri dev
pnpm dev
pnpm fix
pnpm build
pnpm check
```

修改前端代码、配置或依赖前，阅读 `docs/rule-frontend.md`。

## 后端

Rust 后端位于 `src-tauri/`。在该目录中运行 Cargo 命令：

```bash
cd src-tauri
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test
```

从仓库根目录运行时，使用 `--manifest-path src-tauri/Cargo.toml`，例如 `cargo check --manifest-path src-tauri/Cargo.toml --all-targets`。

修改后端代码、配置或依赖前，阅读 `docs/rule-backend.md`。

## 专题文档

- 修改打包或发版流程前，阅读 `docs/releasing.md`。
- 修改 `src-tauri/src/scene/` 或 `src-tauri/src/tasks/archive_scan/` 下的档案库自动化逻辑前，阅读 `docs/archive-automation.md`。
- 修改解包数据处理、档案标题映射或相关数据生成脚本前，阅读 `docs/game-data.md`。

## 本地/个人偏好

`AGENTS.local.md` 是被 Git 忽略的个人偏好文档，在开始工作前读取。如有冲突，以本文件 `AGENTS.md` 为准。
