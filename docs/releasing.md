# 打包构建与发版

## 打包构建

项目只产出**绿色便携 zip**（解压即用，不产出任何安装程序）：

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

### 冒烟测试

把 zip 解压到**干净目录**，双击 `OEA.exe`：

- 应用能正常启动；
- `logs/`、`cache/` 在 exe 旁自动生成（`cache/webview-data/` 为 WebView2 用户数据）；
- 扫描任务能正常加载 OCR 模型（`resources/ocr-models/`）与 `resources/data/prts.json`。

## 发版流程

> **自动化发版**：推送 `v*` tag（如 `v0.1.0`）后，release workflow（`.github/workflows/release.yml`）会自动构建 zip 并发布到 GitHub Releases（自动生成 changelog）。
> **tag 必须与 `src-tauri/tauri.conf.json` 中的 `version` 一致**（例如版本为 `0.1.0` 时打 `v0.1.0`）。workflow 会在构建前校验二者一致，不一致则构建失败。

1. **同步版本号**（三处保持一致，例如要发 `0.1.0`）：
   - `package.json` 的 `version`
   - `src-tauri/tauri.conf.json` 的 `version`
   - `src-tauri/Cargo.toml` 的 `[package] version`
2. **确认资源完整**：检查 `resources/` 子模块内容（尤其未跟踪的 `data/`、`icons/`），必要时先提交到 `oea-resource` 仓库并更新子模块引用。
3. **本地全量检查**：

   ```bash
   pnpm build
   pnpm check

   cd src-tauri
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo fmt --all -- --check
   cargo test
   ```

4. **打包**：`pnpm package`，得到 `releases/OEA-windows-x86_64-v0.1.0.zip`。
5. **冒烟测试**：见上文「冒烟测试」。
6. **打 tag 并推送**：

   ```bash
   git tag v0.1.0
   git push origin main --tags
   ```

   推送后 release workflow 会自动完成构建与 GitHub Release 发布（上传 zip、自动生成 changelog）。

7. **（可选）手动发布**：若 workflow 未自动创建 Release，可在 [Releases](https://github.com/Logical-Byte/open-endfield-assistant/releases) 页面以 `v0.1.0` 手动创建 release，上传 `releases/OEA-windows-x86_64-v0.1.0.zip`，填写变更说明。
