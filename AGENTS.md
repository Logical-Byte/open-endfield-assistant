## 编码规范

### 依赖管理

- 使用 `pnpm add`、`pnpm remove`、`cargo add` 或 `cargo remove` 添加或移除依赖，拒绝手动更改 `package.json` 或 `Cargo.toml` 依赖项
- 除非必要，不要在命令中指定依赖版本。

### 前端

- Linter 使用 `pnpm lint:fix`（修复属性排序错误）和 `pnpm fix`（任意修改后都需要运行）。
- 拒绝写 `anotherFunction(args)` 的简单包装函数。

### 后端

- 在 `src-tauri` 目录中运行 cargo 命令，或使用 `--manifest-path src-tauri/Cargo.toml`。
- Linter 使用 `cargo check`、`cargo fix --allow-dirty` 和 `cargo clippy --fix --allow-dirty`.
- 使用 `Arc::clone(&foo)` 克隆原子引用计数指针，拒绝 `foo.clone()`。
- 在注释中使用反引号 backquote 包裹代码片段。
- `unsafe` 仅包裹单个函数调用表达式。`? ; =` 放在 unsafe 块外。例：`let foo = unsafe { bar(baz) }?;`。
- 后端的运行时行为不得更改“根目录”以外的磁盘文件。“根目录”在开发环境下，指 `package.json` 所在目录。在打包后，指可执行文件所在目录。

## 本地/个人偏好

`AGENTS.local.md` 是被 Git 忽略的个人偏好文档，在开始工作前读取。如有冲突，以本文件 `AGENTS.md` 为准。
