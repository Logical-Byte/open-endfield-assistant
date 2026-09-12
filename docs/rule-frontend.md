# 前端代码规范

修改前端代码、配置或依赖时遵循本文件。

## 依赖管理

- 使用 `pnpm add` 或 `pnpm remove` 添加或移除依赖，拒绝手动更改 `package.json` 依赖项。
- 除非必要，不要在命令中指定依赖版本。

## TypeScript 与函数

- 定义有名字的函数用 `function` 关键字，回调/匿名函数用箭头函数；
- 本项目启用 TypeScript：所有函数参数与返回值都要有类型注解（返回 `void` 的除外）；
- 拒绝写 `anotherFunction(args)` 的简单包装函数。

## 配置的事实来源与生效时机

- `Controller` 持有的后端内存配置是已生效配置的事实来源；
- `oeaConfig` 等前端响应式配置是 UI 展示投影，也可以暂存尚未保存成功的编辑值；
- 编辑值仅在 `save_oea_config` 成功后生效。保存完成前调用后端命令时，命令可以使用上一次成功保存的配置；
- 后端命令自行读取所需配置，并在入口克隆一份调用期间不变的快照。更新命令不接收前端重复传入的代理、凭据或更新源参数。

## Vue 与 Nuxt UI

本项目使用 Vue 和 Vite，并将 Nuxt UI 作为 Vue 组件库使用。沿用现有 Vue/Vite 项目约定实现界面。

- 对按钮、输入框等交互控件，在 Nuxt UI 提供等价组件时使用该组件；布局和文档结构使用合适的语义化 HTML；
- 图标使用 Lucide 图标集提供的 `i-lucide-*` 图标；
- 用 Nuxt UI 组件的 props 表达组件变体和状态，例如 `variant="subtle"`；用 Tailwind CSS 处理布局，以及组件 API 无法表达的样式；
- 用 Nuxt UI 语义色统一明暗主题，例如 `text-toned`、`text-primary`、`bg-muted` 和 `bg-accented`；仅在语义色无法表达固定颜色等实际需求时使用 Tailwind 调色板或自定义颜色；
- 无法确定组件是否存在，或不确定其 props、slots、图标语法、语义色和无障碍行为时，查阅与当前 `@nuxt/ui` 版本匹配的[官方文档](https://ui.nuxt.com/llms.txt)；
- 属性排序等可自动修复的问题，运行 `pnpm lint:fix` 即可。

完成界面修改前，逐一检查本次改动的交互控件、图标、组件样式和颜色是否遵循上述规则，并在最终说明中列出例外及原因。

## 检查与测试

前端改动运行：

```bash
pnpm fix
pnpm build
pnpm check
```
