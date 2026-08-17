# 后端代码规范

修改 `src-tauri/` 下的后端代码、配置或依赖时遵循本文件。

## 平台支持

Windows x86_64 是唯一受支持的运行平台，也是后端功能的验收环境。普通功能开发直接维护 Windows 实现，新代码进入 Windows 路径，并在 Windows 上完成检查与测试。

macOS 开发外壳仅用于按需调试前端和可移植后端能力，不连接或操作真实游戏，也不属于 CI、打包或发布目标。macOS 编译通过和平台条件编译路径只在任务范围明确包含 macOS 外壳维护时成为验收要求。

## 依赖管理

- 使用 `cargo add` 或 `cargo remove` 添加或移除依赖，拒绝手动更改 `Cargo.toml` 依赖项。
- 除非必要，不要在命令中指定依赖版本。

## Cargo 工作目录

在 `src-tauri` 目录中运行 cargo 命令，或使用 `--manifest-path src-tauri/Cargo.toml`。

## Rust 写法

- 使用 `Arc::clone(&foo)` 克隆原子引用计数指针，拒绝 `foo.clone()`。
- 在注释中使用反引号 backquote 包裹代码片段。
- `unsafe` 仅包裹单个函数调用表达式。`? ; =` 放在 unsafe 块外。例：`let foo = unsafe { bar(baz) }?;`。

## 磁盘写入边界

后端的运行时行为不得更改“根目录”以外的磁盘文件。“根目录”在开发环境下，指 `package.json` 所在目录。在打包后，指可执行文件所在目录。路径解析见 `src-tauri/src/app_paths.rs`。

## 自动修复

Linter 使用：

```bash
cargo check
cargo fix --allow-dirty
cargo clippy --fix --allow-dirty
```

完整的提交前检查见 [贡献指南](../CONTRIBUTING.md#检查与测试)。
