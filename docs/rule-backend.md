# 后端代码规范

修改 `src-tauri/` 下的后端代码、配置或依赖时遵循本文件。

## 工具链

- 在 `src-tauri/` 目录中运行 Cargo 命令。从仓库根目录运行时，利用 `--manifest-path src-tauri/Cargo.toml`。
- 使用 `cargo add` 和 `cargo remove` 管理依赖。除非兼容性要求必须锁定版本，否则让 Cargo 选择版本。

## 编译、检查与测试

Windows x86_64 是后端唯一支持的平台和验收环境。CI 只要求 Windows 实现，并通过 Windows 编译和测试。Windows 上的检查与测试：

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

macOS 上的开发、测试和打包不属于验收范围。可以使用 `cargo xwin` 检查 Windows 编译、Clippy 和构建结果。测试以 Windows/CI 结果为准。

## 编码风格

- 使用 `Arc::clone(&value)` 克隆原子引用计数指针。
- 在注释中使用反引号包裹代码片段。
- `unsafe` 块只包裹单个函数调用表达式，赋值、`?` 和分号放在块外。例如：`let value = unsafe { call() }?;`。

## 语义、模块、职责约定

### 应用根目录、相对路径、`AppPaths`

- 只读写*应用根目录*内的文件。开发环境的*应用根目录*是 `package.json` 所在目录。打包后的*应用根目录*是可执行文件所在目录。
- 外部输入的相对于*应用根目录*的路径，应验证其不能使用绝对路径、父目录片段、Windows 路径前缀或符号链接逃逸根目录。最终目标仍位于根目录内的符号链接可以使用。解析已经存在的相对文件时复用 `resolve_existing_relative_file`。
- 避免为了让下游取得某一个路径而层层透传或长期保存 `AppPaths`。调用方应优先解析出具体的 `Path` 或 `PathBuf` 后传入。下列情形例外：
  - 一段内聚流程需要访问多个应用目录、需要保持已经验证或选择的应用根目录。
  - 测试需要注入临时根目录。
  - 对象本身负责一组基于应用根目录的路径布局时，可以传递或保存 `AppPaths`。

### 日志

- `INFO`、`WARN`、`ERROR` 的消息文本面向人阅读，使用中文表述，要求可读性高。
- 调用链、内部状态、标识符等结构化诊断信息主要记录在 `DEBUG` 或 `TRACE` 日志中。底层错误可以保留在结构化 `error` 字段中。

## 后台线程生命周期

- 与应用生命周期绑定的常驻或守护线程，以及相关保活资源，统一由 `BackgroundThreads` 持有和管理。

### 操作系统原语

- `platform::<topic>` 向其他模块提供与操作系统相关的能力。
- `platform::<topic>` 的界面使用平台无关的类型，不得泄露 `windows` crate 的句柄、错误、常量或其他原生类型。
- `windows` crate 的句柄、错误、常量、直接调用及相关的私有实现放在 `platform::windows`。
