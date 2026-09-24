# 打包构建与发版

## 打包构建

项目只产出**绿色便携 zip**（解压即用，不产出任何安装程序）：

应用内更新如何从该 ZIP 构造 candidate 并提交可执行文件与资源，见
[更新安装事务](update-installation.md)。

```bash
pnpm package
```

等价于：

```bash
tauri build --no-bundle && jiti scripts/package.ts
```

- `tauri build --no-bundle`：编译 release 版 exe（前端已内嵌）。`tauri.conf.json` 中 `bundle.targets` 为空数组，因此**不会**生成 NSIS/MSI 安装程序。
- `scripts/package.ts`：组装并打 zip：
  1. 读取 `src-tauri/tauri.conf.json` 的 `productName` / `version` 与 `process.arch`，得到产物名 `OEA-windows-x86_64-v0.1.0.zip`；
  2. 将 `src-tauri/target/release/oea.exe`（`--no-bundle` 下二进制沿用 Cargo 包名，需自行重命名）重命名为 `OEA.exe`，连同 `resources/`（含 `ocr-models/`）拷入暂存目录（跳过 `.` 开头的条目，如子模块 `.git`）；
  3. 使用 `yazl` 打 zip。

产物：`releases/OEA-windows-x86_64-v0.1.0.zip`。实际文件名中的版本号和架构取决于当前配置与构建环境；`releases/` 目录已 git 忽略。

## 发版流程

1. 拉取 `main` 分支。
2. 更新解包库。
3. 运行 `pnpm makedata`，然后提交。
   resource 库提交消息：`2026-09-24 更新档案数据`；
   主库提交消息：`feat: 更新档案数据`。
4. 更新版本号：`pnpm bump:version`。
5. （可选）打包：`pnpm package`，然后测一下能不能用。
6. 推送：`git push && git push --tags`。

> **自动化发版**：推送 `v*` tag（如 `v0.1.0`）后，release workflow（`.github/workflows/release.yml`）只构建一次 zip，并独立发布到 GitHub Releases、Cloudflare R2 与 MirrorChyan。R2 的稳定下载入口为 `https://oea.oem.re/` 和 `https://oea.oem.re/latest`。
> **tag 必须与 `src-tauri/tauri.conf.json` 中的 `version` 一致**（例如版本为 `0.1.0` 时打 `v0.1.0`）。workflow 会在构建前校验二者一致，不一致则构建失败。

## R2 发布与回滚

- bucket 固定为 `opendfieldmap-package`，OEA 版本对象位于 `releases/oea/<tag>/`，永不覆盖或自动过期；其他应用使用各自的 `releases/<app-id>/` 命名空间。
- `channels/oea/stable.json` 是 OEA 稳定通道的唯一指针；普通发布只允许指针保持不变或升级，较旧 tag 的乱序任务只能补充归档，不能降低 stable。
- 安装包达到 `512 MiB` 时 R2 发布会失败并保留旧 stable，避免产生无法被当前 Cloudflare 套餐缓存的下载链接。
- 在 GitHub Actions 手动运行 **Build and release**：
  - `operation=publish`：从指定 tag 重建、校验并发布到 R2，不重复创建 GitHub Release。
  - `operation=promote`：校验 R2 中已有版本及其 checksum 后，仅切换 stable，用于受保护的回滚。
- GitHub Environment `r2-production` 需要变量 `R2_ACCOUNT_ID`，以及 secrets `R2_ACCESS_KEY_ID`、`R2_SECRET_ACCESS_KEY`。凭据只能授予 `opendfieldmap-package` 的 R2 Object Read & Write 权限。
