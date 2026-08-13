---
name: nuxt-ui-expert
description: |
  当编写用户界面时，必须使用此技能。优先使用 Nuxt UI 提供的组件、语义化颜色和 Lucide 图标。
---

# Nuxt UI Expert

### 不与 Nuxt 一起使用

本项目使用 Vue 作为前端框架，使用 Nuxt UI 作为组件库。请不要将 Nuxt UI 与 Nuxt 混为一谈，本项目不使用 Nuxt 框架。

### 参阅官方文档

如有需要，请阅读 [llms.txt](https://ui.nuxt.com/llms.txt)。

### 组件优先级

优先使用 Nuxt UI 提供的组件（如 `UButton`, `UInput`, `UInputNumber`, `UBadge`, `UFileUpload` 等），而不是原生 HTML 元素。

### 图标优先级

优先使用 Lucide 图标（`i-lucide-*`）。

### 样式优先级

优先使用 Nuxt UI 组件的 props（如 `variant="subtle"`）来控制样式，而不是直接使用 Tailwind CSS 类。

### 颜色类优先级

优先使用 Nuxt UI 提供的语义化颜色（如 `text-toned` `text-primary` `bg-muted` `bg-accented`），尽量避免使用 Tailwind 颜色表中的颜色类（如 `text-gray-800 dark:text-gray-200` `bg-slate-100 dark:bg-slate-800`）。

优先使用 Nuxt UI 提供的语义化颜色来统一深浅色主题，避免为深浅色主题分别写不同的颜色类（如 `text-gray-800 dark:text-gray-200`）。

但是，如果确实需要固定的颜色，或者 Nuxt UI 提供的语义化颜色不满足需求，经慎重考虑后可以使用 Tailwind 的颜色类，也可以写自定义的颜色。
