# 贡献指南

欢迎参与 **Open Endfield Assistant（OEA）** 的开发与维护！这份指南帮助你了解项目、准备开发环境并提交贡献。

## 项目概览

OEA 的默认桌面入口使用 Rust 与 GPUI，直接调用共享的自动化、OCR、设置和更新逻辑。原有 Tauri 2 与 Vue 3 前端保留为 `legacy-tauri` 入口，供迁移期间回归比较。原生入口与后台验证方式见 [GPUI 迁移](docs/gpui-port.md)。

- `src/`：前端界面与交互。
- `Cargo.toml`：Rust workspace，统一管理根目录的 `Cargo.lock` 与 `target/`。
- `crates/oea-cli/`：独立开发者 CLI，使用方式见[开发者 CLI](docs/developer-cli.md)。
- `crates/oea/`：[Rust](https://www.rust-lang.org/) 桌面后端。
- `resources/`：前后端共享的模板图、游戏数据和图标，通过 [Git](https://git-scm.com/) submodule 管理。
- `scripts/`：数据生成、打包等脚本。

## 开始贡献

1. 按 [环境配置与本地调试](docs/setup.md) 准备工具链、代码和依赖，并启动应用。仅调试前端或查看日志的方式也在这份文档中。
2. 在 [GitHub](https://github.com/Logical-Byte/open-endfield-assistant) 上提出或认领 Issue，说明你想做的工作。分支、Commit 和 PR 的约定见 [协作规范](docs/conventions.md)。
3. 根据修改范围，参考 [前端规范](docs/rule-frontend.md) 或 [后端规范](docs/rule-backend.md) 进行开发，并完成其中的检查。
4. 提交 PR，介绍改动内容与验证情况，方便其他贡献者理解和审阅。

## 专题文档

- [打包与发版](docs/releasing.md)：便携包构建、发版与回滚流程。
- [档案库自动化](docs/archive-automation.md)：界面识别、导航与扫描流程。
- [数据与共享类型管线](docs/data-pipeline.md)：资源生成、后端投影与前端共享类型。
