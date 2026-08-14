/// <reference types="vite/client" />

/** 构建期注入的 GitHub 只读 token（来自 `.env` 的 `GITHUB_TOKEN`；仓库 public 后移除）。 */
declare const __OEA_GITHUB_TOKEN__: string;
