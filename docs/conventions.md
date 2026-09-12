# 协作规范

- Issue：在 [GitHub](https://github.com/Logical-Byte/open-endfield-assistant) 上说明要做什么，或认领已有 Issue。
- 分支：从 `main` 切出功能分支，例如 `<your_name>/feat/xxx`。
- Commit：建议遵循 Conventional Commits（`feat:`、`fix:`、`refactor:`、`docs:` 等），保持简洁清晰。
- PR：描述改动内容与验证情况；提交前通过[前端检查](rule-frontend.md#检查与测试)和[后端检查](rule-backend.md#检查与测试)。

## Git hooks

提交时检查暂存文件，推送时按改动范围运行检查，具体配置见 [lefthook.yml](../lefthook.yml)。hooks 只检查、不自动暂存修复结果，避免改变分块暂存的内容。
