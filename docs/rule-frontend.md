# 前端代码规范

修改前端代码、配置或依赖时遵循本文件。

## 依赖管理

- 使用 `pnpm add` 或 `pnpm remove` 添加或移除依赖，拒绝手动更改 `package.json` 依赖项。
- 除非必要，不要在命令中指定依赖版本。

## TypeScript 与函数

- 定义有名字的函数用 `function` 关键字，回调/匿名函数用箭头函数；
- 本项目启用 TypeScript：所有函数参数与返回值都要有类型注解（返回 `void` 的除外）；
- 拒绝写 `anotherFunction(args)` 的简单包装函数。

## Vue 组件

- 需要编写组件时遵循 `nuxt-ui-expert` 规范；
- 属性排序等可自动修复的问题，运行 `pnpm lint:fix` 即可。

## 自动修复

任意修改后都需要运行：

```bash
pnpm fix
```

完整的提交前检查见 [贡献指南](../CONTRIBUTING.md#提交前检查)。
