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

## Vue 与 Nuxt UI 编码风格

本项目使用 Vue 和 Vite，并将 Nuxt UI 作为 Vue 组件库使用。

- 按钮、输入框等交互控件，在 Nuxt UI 提供等价组件时使用该组件。布局和文档结构使用合适的语义化 HTML。
- 图标使用 Lucide 图标集提供的 `i-lucide-*` 图标。
- 用 Nuxt UI 组件的 props 表达组件变体和状态，例如 `variant="subtle"`。用 Tailwind CSS 处理布局，以及组件 API 无法表达的样式。
- 用 Nuxt UI 语义色统一明暗主题，例如 `text-toned`、`text-primary`、`bg-muted` 和 `bg-accented`。仅在语义色无法表达固定颜色等实际需求时使用 Tailwind 调色板或自定义颜色。
- 必要时查阅 `@nuxt/ui` 的[官方文档](https://ui.nuxt.com/llms.txt)。

不符合此处约定的 Vue 与 Nuxt UI 编码风格的，在 PR 中应列出例外和原因。

## 语义、模块、职责约定

### 配置的事实来源与生效时机

- `Controller` 持有的后端内存配置是已生效配置的事实来源。
- `settingsDraft` 仅供设置编辑 UI 读写和暂存未保存的编辑；前端业务逻辑只读取最近一次成功保存的 `effectiveSettings`，不得依赖 draft 做决策。
- 编辑值仅在 `save_oea_config` 成功后生效。保存完成前调用后端命令时，命令可以使用上一次成功保存的配置。
- 后端命令自行读取所需配置，并在入口克隆一份调用期间不变的快照。
