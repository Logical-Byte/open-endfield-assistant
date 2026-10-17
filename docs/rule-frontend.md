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

### 图标离线打包

`vite.config.ts` 中的 Nuxt UI `icon.clientBundle.scan` 扫描 `src/**/*.{vue,ts}`，从本地安装的 `@iconify-json/*` 图标集中提取图标数据并打入前端 JS。Nuxt UI 默认图标也会自动打包。

- 新增 Lucide 图标时，在 Vue 或 TS 中写完整名称，例如 `i-lucide-heart`。模板、三元表达式、对象映射和 toast 中的完整字符串都可被扫描，无需维护另一份图标清单。
- 使用新图标集时，通过 `pnpm add -D @iconify-json/<collection>` 安装对应图标集。构建仅打包扫描到的图标及 Nuxt UI 默认图标。
- 扫描不会执行代码。避免用字符串拼接生成图标名称，例如 `'i-lucide-' + name`。需要这种写法时，在 `icon.clientBundle.icons` 中显式列出所有可能名称，使用 `lucide:heart` 格式。
- 图标定义放在 `src` 下的 Vue 或 TS 文件中。若迁移到其他目录或文件类型，同时调整 `scan.globInclude`。
- 构建后清空浏览器存储并阻断外部资源，检查图标渲染。扫描发现但无法解析的名称可能被忽略，不能仅凭构建成功认定图标已打包。

## 界面文案

- 面向用户的界面不展示来自解包数据的内部标识符，例如档案 ID。
- 禁止使用 `·` 做文案分隔符。这通常意味着罗列一串名词。使用单个名词，或者完整的句子。

### i18n 文案维护

- 应用文案使用 UiLocale。游戏标题、分类和识别文本使用 GameLocale，两者独立。
- 关闭跨语言回退。缺译显示原始 key 并记录当前语言。

i18next 的用法见[官方文档](https://www.i18next.com/)。

## 语义、模块、职责约定

### 设置的事实来源与生效时机

- `Controller` 持有的后端内存设置是已生效设置的事实来源。
- `draftSettings` 是只读的 UI 编辑投影，通过 `editSettings` 提交修改；`effectiveSettings` 是最近一次成功加载或保存的只读投影，供更新编排等业务流程读取。
- 加载完成前禁用持久化设置控件；加载失败时保持禁用并提供重试，前端默认值只用于占位。
- 编辑值仅在 `save_oea_settings` 成功后生效。保存完成前调用后端命令时，命令可以使用上一次成功保存的设置。
- 后端命令自行读取所需设置，并在入口克隆一份调用期间不变的快照。
