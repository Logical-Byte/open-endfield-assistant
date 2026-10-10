# 前端代码规范

修改前端代码、配置或依赖时遵循本文件。

## 工具链

- 使用 `pnpm add` 或 `pnpm remove` 添加或移除依赖，拒绝手动更改 `package.json` 依赖项。
- 除非必要，不要在命令中指定依赖版本。

## 编译、检查与测试

前端改动运行：

```bash
pnpm fix
pnpm build
pnpm check
```

## TS 编码风格

- 定义有名字的函数用 `function` 关键字，回调/匿名函数用箭头函数。
- 本项目启用 TypeScript：所有函数参数与返回值都要有类型注解（返回 `void` 的除外）。
- 拒绝写 `anotherFunction(args)` 的简单包装函数。
- 新增或迁移后端共享类型时，通过 [ts-rs 管线](data-pipeline.md#ts-rs-编写与管理约定) 生成并从手写 barrel 导入。生成文件禁止手改，前端展示状态与运行时选项继续放在所属 feature。

## Vue 与 Nuxt UI 编码风格

本项目使用 Vue 和 Vite，并将 Nuxt UI 作为 Vue 组件库使用。

- 按钮、输入框等交互控件，在 Nuxt UI 提供等价组件时使用该组件。布局和文档结构使用合适的语义化 HTML。
- 图标使用 Lucide 图标集提供的 `i-lucide-*` 图标。
- 用 Nuxt UI 组件的 props 表达组件变体和状态，例如 `variant="subtle"`。用 Tailwind CSS 处理布局，以及组件 API 无法表达的样式。
- 用 Nuxt UI 语义色统一明暗主题，例如 `text-toned`、`text-primary`、`bg-muted` 和 `bg-accented`。仅在语义色无法表达固定颜色等实际需求时使用 Tailwind 调色板或自定义颜色。
- 必要时查阅 `@nuxt/ui` 的[官方文档](https://ui.nuxt.com/llms.txt)。

不符合此处约定的 Vue 与 Nuxt UI 编码风格的，在 PR 中应列出例外和原因。

## 界面文案

- 禁止使用 `·` 做文案分隔符。这通常意味着罗列一串名词。使用单个名词，或者完整的句子。

### i18n 文案维护

以下约定用于 i18n 接入及后续文案迁移。迁移按功能逐步完成。

- 应用文案使用 UiLocale。游戏标题、分类和识别文本使用 GameLocale，两者独立。
- 翻译资源集中放在 `src/shared/i18n/locales/zh-CN.json` 和 `en.json`，每种语言一份 JSON。使用扁平的语义 key，例如 `settings.saveFailed.title`，按 feature namespace 分组。通用操作可使用 `common` namespace，语境不同的文案分别定义。
- key 按完整字符串进行大小写敏感的升序排序，排序不依赖系统语言。检查与排序工具放在 `scripts/checks/i18n.ts`，检查模式接入 `pnpm check`，排序修复模式接入 `pnpm fix`。缩进和换行交给 Oxfmt。
- 新增或修改文案时，在同一个 PR 中维护中英文。中文资源作为结构 schema 来源，英文保持对应 key。审核文案含义、术语、参数和页面展示，自动检查 key 对齐、非空消息、消息语法和命名插值参数。中英文复数分支数量可以不同。
- 使用完整句子与命名参数，英文复数由 Vue I18n 处理。避免拼接翻译片段，路径、版本号、ID、宽高等参数保留原始事实。翻译消息不写 HTML，需要链接或强调时由 Vue 组件表达结构。
- 调用端通过类型检查约束 key。动态 key 使用有类型的有限映射，不通过任意字符串拼接构造。只比较中英文 key 不能发现两种语言同时遗漏的调用项。
- 两种语言均完整交付，关闭跨语言回退。意外缺译时显示原始 key 并记录 locale 与 key，保持业务状态，不以语言回退掩盖缺译。
- 长期展示的状态、错误和选项保存业务事实，在模板或 computed 中按当前 UiLocale 生成文案。普通 TS 模块在展示时调用全局翻译入口，不在模块初始化时固化翻译字符串。
- 所有仍在显示的 toast 随 UiLocale 原地更新标题、描述及操作按钮文案。保留通知 ID、开关状态、业务回调和消失计时，已关闭的通知不重新出现。通知翻译统一维护，不按短暂和长期通知分别实现。
- 日期数字格式集中定义在 `src/shared/i18n/formats.ts`。中文使用 `zh-CN`，英文使用 `en-US`，日期包含年月日，时间使用系统时区及 24 小时制。语言切换只改变展示，不改变时间戳。持续时长和文件大小使用各自明确的单位规则，版本号、路径、ID 和分辨率按原始值展示。

实施决议见 [确定前端翻译资源组织与切换实现方案](https://github.com/Logical-Byte/open-endfield-assistant/issues/214)。

## 语义、模块、职责约定

### 设置的事实来源与生效时机

- `Controller` 持有的后端内存设置是已生效设置的事实来源。
- `draftSettings` 是只读的 UI 编辑投影，通过 `editSettings` 提交修改；`effectiveSettings` 是最近一次成功加载或保存的只读投影，供更新编排等业务流程读取。
- 加载完成前禁用持久化设置控件；加载失败时保持禁用并提供重试，前端默认值只用于占位。
- 编辑值仅在 `save_oea_settings` 成功后生效。保存完成前调用后端命令时，命令可以使用上一次成功保存的设置。
- 后端命令自行读取所需设置，并在入口克隆一份调用期间不变的快照。
