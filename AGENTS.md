## 添加依赖的规则

当需要添加依赖时，必须使用 `pnpm add` 或者 `cargo add` 命令。除非必要，在命令中不要指定依赖的版本。不得修改 `package.json` 或者 `Cargo.toml` 文件中的依赖项。需要移除依赖时同理。

## 前端编写规范

- 如果遇到属性排序错误，不需要您手动修复，运行 `pnpm lint:fix` 即可自动修复。
- 不要为 URL 构建函数添加无意义的包装层——如果函数只是 `return anotherFunction(args)`，直接使用后者。
- 适度重构：确保代码可读性和可维护性。
- 如果需要编写组件，请遵循 `nuxt-ui-expert` skill。

## 后端编写规范

- 使用 cargo 命令时注意工作目录。
- 原子引用计数的克隆必须写 `Arc::clone(&x)`，不能写 `x.clone()`。
- 改完代码后，运行 `cargo check`，确保编译通过。编译通过后，运行 `cargo fix --allow-dirty` 和 `cargo clippy --fix --allow-dirty`，确保代码风格符合要求。
