# 协作规范

- Issue：在 [GitHub](https://github.com/Logical-Byte/open-endfield-assistant) 上说明要做什么，或认领已有 Issue。
- 分支：从 `main` 切出功能分支，例如 `<your_name>/feat/xxx`。
- Commit：遵循 Conventional Commits（`feat:`、`fix:`、`refactor:`、`docs:` 等）。类型与 scope 使用英文，标题中的说明和正文使用中文，例如 `feat(backend:navigation): 添加导航能力`。
- PR：按下文约定描述最终交付，提交前按改动范围通过 [前端检查](rule-frontend.md#编译检查与测试) 和 [后端检查](rule-backend.md#编译检查与测试)。

## 协作语言

仓库文档、Issue、PR 的正文与评论使用中文。代码符号、库名和工具名保留原文，无需额外添加译名。领域概念遵循[术语表](GLOSSARY.md)及其[使用约定](agents/domain.md)。

## 本地忽略规则

仅适用于个人机器或工作树的忽略规则自行写入机器本地的 `.git/info/exclude`。项目开发普遍产生的文件由仓库 `.gitignore` 管理。

## PR 正文

- 说明要解决的问题、最终交付的行为，以及评审所需的设计取舍。正文以最终改动为准，省去模板占位内容、未进入最终实现的讨论等与评审无关的信息。
- 验证部分记录有助于评审的结果、限制和 CI 未覆盖的检查，省去 CI 已展示的常规检查清单。
- 涉及界面变化的改动，建议提供改动前后的截图或录屏，展示有意义的视觉结果。

## PR 合并

- 默认通过一个 merge commit 合并 PR，保留分支内的提交历史。如果 PR 含有较多 fixup 提交、历史不适合保留，或包含同步主线等 merge commit 时，使用 squash merge。
- 单个 PR 的标题和正文准确描述交付内容时，使用仓库默认生成的合并信息。
- 需要自定义合并信息时，标题使用 Conventional Commits，例如 `feat(foo): 添加 bar (#123)`。正文保留 PR 中尚未记录在其他文档里的重要动机与决策。
- 将整组 stacked PR 合入一个 merge commit 时，自定义标题和正文，覆盖所有纳入的 PR、动机与重要决策。GitHub 默认信息可能只描述最上层 PR，合并前确认所用工具支持自定义信息。
- 各层 PR 有独立交付价值时，可以按从底到顶的顺序分别合并，保留各自的合并记录。

## 文档格式

中文行文优先用句号拆分长句，避免使用中文分号 `；`。
禁止新增 Markdown 表格。使用段落或列表组织内容，避免表格列宽对齐在修改时产生大段 diff。
