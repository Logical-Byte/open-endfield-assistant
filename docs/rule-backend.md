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

进行 vision 开发、ROI 标定等工作时，可以考虑利用 [`resources/dev/`](../resources/dev/README.md) 中的截图资源进行初步验证。

## 编码风格

- 使用 `Arc::clone(&value)` 克隆原子引用计数指针。
- 在注释中使用反引号包裹代码片段。
- `unsafe` 块只包裹单个函数调用表达式，赋值、`?` 和分号放在块外。例如：`let value = unsafe { call() }?;`。
- 胶水层负责跨模块编排。引用 `Status`、`Config`、`Runtime`、`Kind` 等难以辨认职责的类型时，保留所属模块的命名空间，例如 `automation::Runtime`。`UiLocale`、`GameLocale` 等名称已清晰表达职责的类型可以直接导入，模块内部也可以使用短名称。
- 跨模块类型或模块通过 `use` 导入，类型使用处不保留 `crate::` 前缀。命名空间用于说明职责和避免歧义，无需在所有跨模块类型前添加。
- 新增或迁移 Tauri 边界的数据类型时，按 [ts-rs 编写与管理约定](data-pipeline.md#ts-rs-编写与管理约定) 从 Rust 定义生成前端类型，并提交生成文件。
- 模块应尽量提供窄 Interface，避免调用者依赖内部子模块结构或其他 Implementation 细节。原因：降低调用者的心智负担和模块之间的耦合，使内部实现与代码布局可以局部调整。

## 语义、模块、职责约定

### 视觉识别与领域识别

- `vision` 只实现基础、通用的图像识别 primitive，例如“这个区域是什么文字”“这个区域匹配哪个模板”“这个区域的亮度是多少”以及区域颜色判断。调用方传入图片、ROI、阈值、候选模板等参数，`vision` 返回通用的识别或统计结果。
- “这个档案的标题是什么”“这个 UI 的按钮是启用了还是没有启用”属于领域特定逻辑，由对应的领域或工作流模块实现。这些逻辑通常带有针对具体界面的默认 ROI、阈值、模板和领域类型，负责调用 `vision` 并解读其输出。例如，档案标题识别选择标题区域并将 OCR 文字匹配到档案目录，按钮状态识别选择按钮区域并将亮度或模板匹配结果解释为启用状态。
- 领域专属的布局、识别参数和结果解释保留在调用方，`vision` 不依赖领域类型。通用算法可以拥有算法自身的默认参数。

### 应用根目录、相对路径、`AppPaths`

- 只读写*应用根目录*内的文件。开发环境的*应用根目录*是 `package.json` 所在目录。打包后的*应用根目录*是可执行文件所在目录。
- `cli` feature 的开发者命令按 [Developer CLI](developer-cli.md) 规则读写用户指定的文件系统路径，相对路径基于当前工作目录。这一例外仅适用于 CLI 的显式图片、模板和截图输出，不改变生产 `AppPaths` 资源发现与模板缓存的根目录约束。
- 外部输入的相对于*应用根目录*的路径，应验证其不能使用绝对路径、父目录片段、Windows 路径前缀或符号链接逃逸根目录。最终目标仍位于根目录内的符号链接可以使用。解析已经存在的相对文件时复用 `resolve_existing_relative_file`。
- 避免为了让下游取得某一个路径而层层透传或长期保存 `AppPaths`。调用方应优先解析出具体的 `Path` 或 `PathBuf` 后传入。下列情形例外：
  - 一段内聚流程需要访问多个应用目录、需要保持已经验证或选择的应用根目录。
  - 测试需要注入临时根目录。
  - 对象本身负责一组基于应用根目录的路径布局时，可以传递或保存 `AppPaths`。

### 日志

- `INFO`、`WARN`、`ERROR` 的消息文本面向人阅读，使用中文表述，要求可读性高。
- 调用链、内部状态、标识符等结构化诊断信息主要记录在 `DEBUG` 或 `TRACE` 日志中。底层错误可以保留在结构化 `error` 字段中。

### 后台线程生命周期

- 与应用生命周期绑定的常驻或守护线程，以及相关保活资源，统一由 `BackgroundThreads` 持有和管理。

### 操作系统原语

- `platform::<topic>` 向其他模块提供与操作系统相关的能力。
- `platform::<topic>` 的界面使用平台无关的类型，不得泄露 `windows` crate 的句柄、错误、常量或其他原生类型。
- `windows` crate 的句柄、错误、常量、直接调用及相关的私有实现放在 `platform::windows`。
