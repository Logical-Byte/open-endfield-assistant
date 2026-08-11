## 添加依赖的规则

当需要添加依赖时，必须使用 `pnpm add` 或者 `cargo add` 命令。除非必要，在命令中不要指定依赖的版本。不得修改 `package.json` 或者 `Cargo.toml` 文件中的依赖项。需要移除依赖时同理。

## 前端编写规范

- 如果遇到属性排序错误，不需要您手动修复，运行 `pnpm lint:fix` 即可自动修复。

- 适度重构：确保代码可读性和可维护性。

- 改完代码后，运行 `pnpm fix`，确保代码风格符合要求。

- 如果需要编写组件，请遵循 `nuxt-ui-expert` skill。

- 不要添加无意义的包装层：如果函数只是 `return anotherFunction(args)`，直接使用后者。

## 后端编写规范

- 使用 cargo 命令时注意工作目录，您可以使用 `cargo command --manifest-path src-tauri/Cargo.toml`。

- 适度重构：确保代码可读性和可维护性。

- 改完代码后，运行 `cargo check`，确保编译通过。编译通过后，运行 `cargo fix --allow-dirty` 和 `cargo clippy --fix --allow-dirty`，确保代码风格符合要求。

- 项目根目录定义为（开发时：`package.json` 所在目录，打包后：exe 文件所在目录）。所有磁盘写操作都必须严格限定在项目根目录内，绝对禁止在项目根目录以外的任何目录进行磁盘写操作。

- 原子引用计数的克隆必须写 `Arc::clone(&x)`，不能写 `x.clone()`。

- 注释中的代码片段必须使用反引号包裹，无论是行内注释还是文档注释。

- 所有 unsafe 函数的调用必须将 `unsafe` 块的范围缩小到最小，仅在函数调用表达式的外层添加 `unsafe` 块，问号、分号、赋值等任何其他操作符都必须放在 `unsafe` 块外部，例如 `let result = unsafe { function(argument) }?;`。
