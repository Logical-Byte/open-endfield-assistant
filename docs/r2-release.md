# R2 发布基础设施首次配置

## 对外端点

- OEA 稳定入口：`https://oea.oem.re/` 与 `https://oea.oem.re/latest`。Worker 从 `channels/oea/stable.json` 读取清单并返回不缓存的 `302`。
- OEA 版本化下载：`https://package.oem.re/releases/oea/<tag>/OEA-windows-x86_64-<tag>.zip`，例如 `https://package.oem.re/releases/oea/v0.1.3/OEA-windows-x86_64-v0.1.3.zip`。
- R2 对象 key 与 CDN 路径一一对应；未来应用使用 `releases/<app-id>/<tag>/...` 和 `channels/<app-id>/stable.json`，互不覆盖。

本项目的 release workflow 只使用 `r2-production` Environment 中的三项配置：

- Variable `R2_ACCOUNT_ID`：Cloudflare account ID；
- Secret `R2_ACCESS_KEY_ID`；
- Secret `R2_SECRET_ACCESS_KEY`。

后两项必须来自 Cloudflare R2 API Token。创建 token 时选择 **Object Read & Write**，并且只勾选 `opendfieldmap-package` bucket；不要使用 Admin Read & Write，也不要把 token 放入仓库变量、日志或本地提交。

### 生成 R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY

在 Cloudflare Dashboard 中进入 **R2 → Overview → Manage R2 API Tokens → Create API token**：

1. 选择仅用于 R2 S3 API 的 token，权限设为 **Object Read & Write**。
2. 将资源范围限制为本账号的 `opendfieldmap-package` bucket，不授予 bucket 管理、Zone、Workers 或其他 bucket 权限。
3. 创建后立即复制 **Access Key ID** 和 **Secret Access Key**；Secret 只显示一次，丢失后只能撤销并重新创建。
4. 在 GitHub 仓库 **Settings → Environments → r2-production** 中保存：
   - Secret `R2_ACCESS_KEY_ID` = Access Key ID；
   - Secret `R2_SECRET_ACCESS_KEY` = Secret Access Key；
   - Variable `R2_ACCOUNT_ID` = Cloudflare 账号 ID（不是 Zone ID）。

发布脚本使用 S3 endpoint `https://<R2_ACCOUNT_ID>.r2.cloudflarestorage.com`、region `auto`，bucket 名称在代码中固定为 `opendfieldmap-package`，因此不会因环境变量误配置而写入其他 bucket。建议给 Environment 配置 required reviewer，并每 90 天轮换密钥；轮换时先添加并验证新密钥，再撤销旧密钥。

## Cloudflare 配置

1. 在同一 Cloudflare account 中确认 R2 bucket `opendfieldmap-package` 已存在，并连接 Custom Domain `package.oem.re`。`r2.dev` 保持关闭，最低 TLS 建议为 1.2。
2. 在 Atlos 的 `talos/oem-relink` 目录部署 Worker。`wrangler.toml` 中的 `OEA_PACKAGES` binding 必须指向上述 bucket。
3. `oem.re` zone 当前存在外部管理的 `oem.re/*`、`beta.oem.re/*`、`blog.oem.re/*`、`oea.oem.re/*` routes；不要重新添加 `*.oem.re/*` wildcard，否则会拦截 `package.oem.re` 的 R2 Custom Domain。
4. 为 `package.oem.re/releases/oea/*` 创建 Cache Rule：启用 Cache Everything，Edge TTL 和 Browser TTL 均为一年；不要匹配 `channels/oea/stable.json`。版本对象本身也带有 `public, max-age=31536000, immutable`。未来应用沿用 `releases/<app-id>/...` 与 `channels/<app-id>/stable.json` 命名空间。
5. 为 `oem.re` zone 启用 Smart Tiered Cache。R2 Custom Domain 必须走 proxied Cloudflare DNS；不要为生产下载 CNAME 到 `r2.dev`。

## GitHub 配置

在仓库 **Settings → Environments → r2-production** 中添加上述变量和 secrets。建议启用 required reviewers，并把 Environment 的部署分支/Tag policy 限制到受保护的 `v*` 发布 tag 和受保护主分支的手动运行。

推送版本 tag 后，`Build and release` 会构建一次 zip，再由独立 job 发布到 GitHub Release、R2 和 MirrorChyan。首次导入既有版本时手动运行：

- `operation=publish`、`tag=v0.1.2`：重建并上传版本对象，然后推进 stable；
- `operation=promote`、`tag=v0.1.2`：只验证并切换已有 R2 对象，用于回滚。

OEA 版本对象位于 `releases/oea/<tag>/`，稳定指针是 `channels/oea/stable.json`。普通发布不会覆盖对象或倒退 stable；回滚必须通过受保护的 `promote` 手动运行。其他应用应使用自己的 `<app-id>` 命名空间，避免对象和 stable 指针冲突。即使对象已存在且 metadata、大小一致，发布和 promote 仍会重新读取对象计算 SHA-256，防止内容被替换后错误进入 stable。

## 验收命令

```bash
curl -I https://oea.oem.re/
curl -I https://oea.oem.re/latest
curl -I https://package.oem.re/releases/oea/v0.1.2/OEA-windows-x86_64-v0.1.2.zip
```

稳定入口应返回 `302`、`Cache-Control: no-store`，并将 Location 指向 `package.oem.re` 的版本化 URL；版本 zip 应返回 `Content-Type: application/zip`、长期 immutable 缓存头，并在新版本发布后保持旧 URL 可用。
