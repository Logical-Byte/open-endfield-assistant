# 后端代码规范

修改 `src-tauri/` 下的后端代码、配置或依赖时遵循本文件。

## 工具链

- 使用 `cargo add` 和 `cargo remove` 管理依赖。除非兼容性要求必须锁定版本，否则让 Cargo 选择版本；
- 在 `src-tauri/` 目录中运行 Cargo 命令。从仓库根目录运行时，通过 `--manifest-path src-tauri/Cargo.toml` 指定 manifest；
- 开发过程中先运行 `cargo check` 获取编译反馈。使用 `cargo fix --allow-dirty` 和 `cargo clippy --fix --allow-dirty` 应用 Rust 自动修复。

## 检查与测试

提交前，在 Windows 的 `src-tauri/` 目录中运行：

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

## 编译

Windows x86_64 是后端唯一支持的平台和验收环境。新功能只需提供 Windows 实现，并通过 Windows 编译和测试；macOS 开发外壳不属于验收范围，无需为新功能补充 macOS 实现或维持 macOS 编译兼容。

在非 Windows 环境中可以使用 `cargo xwin` 检查 Windows 编译、Clippy 和构建结果。交叉编译不能执行 Windows 测试，运行时行为仍以 Windows 环境中的测试结果为准。

## 编码风格

- 使用 `Arc::clone(&value)` 克隆原子引用计数指针；
- 在注释中使用反引号包裹代码片段；
- `unsafe` 块只包裹单个函数调用表达式，赋值、`?` 和分号放在块外。例如：`let value = unsafe { call() }?;`。

## 语义与行为规则

- 后端运行时只读写应用根目录内的文件。开发环境的根目录是 `package.json` 所在目录，打包后的根目录是可执行文件所在目录；统一通过 `src-tauri/src/app_paths.rs` 获取这些路径；
- 将外部输入的相对路径与根目录拼接前，验证其不能使用绝对路径、父目录片段、Windows 路径前缀或符号链接逃逸根目录。最终目标仍位于根目录内的符号链接可以使用；解析已经存在的相对文件时复用 `resolve_existing_relative_file`；
- Windows API 的原生类型、常量、直接调用及其私有实现放在 `platform::windows`；
- 通过 `platform::<topic>` 向其他模块提供平台能力。接口使用项目自有或平台无关的类型，调用方不得依赖 `windows` crate 的句柄、错误、常量或其他原生类型。
