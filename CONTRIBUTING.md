# Contributing Guide

欢迎参与 **Open Endfield Assistant（OEA）** 的开发。OEA 是一个活跃的社区项目，欢迎每一位大佬参与开发和维护。

本文档面向开发者，涵盖开发环境准备、参与方式、本地调试、打包构建与发版流程。

用户使用相关的内容见仓库根目录的 [README.md](README.md)。

## 项目概览

- **技术栈**：前端 Vue 3 + Nuxt UI（Vue 版）+ Vite + TypeScript；桌面壳 Tauri 2（Rust）；OCR 使用 RapidOCR（ONNX 模型，位于 `models/`）。
- **仓库结构**：
  - `src/` — 前端（Nuxt UI Vue 版）
  - `src-tauri/` — Rust 后端（Tauri 2）
  - `resources/` — git submodule（`Logical-Byte/oea-resource`），前后端共享资产（模板图、游戏数据、图标）
  - `models/` — OCR ONNX 模型（约 32MB，已 git 忽略）
  - `scripts/` — 数据生成（`makeAllData.ts`）、打包（`package.ts`）等脚本
  - `.github/workflows/ci.yml` — CI（前端 check + 后端 fmt/clippy/test）
- **绿色便携**：应用所有磁盘写入都限定在应用目录内（开发期为项目根，发布版为 exe 所在目录），路径解析见 `src-tauri/src/app_paths.rs`。

## 开发环境准备

| 依赖             | 版本要求                                        | 说明                           |
| ---------------- | ----------------------------------------------- | ------------------------------ |
| Node.js          | ≥ 20.19（建议 22 LTS）                          | Vite 7 要求                    |
| pnpm             | 11.x（仓库锁定 `packageManager: pnpm@11.17.0`） | 建议启用 corepack              |
| Rust             | ≥ 1.85（stable）                                | 仓库使用 `edition = "2024"`    |
| Windows          | 10 / 11（x64）                                  | 当前仅面向 Windows x86_64 分发 |
| WebView2 Runtime | 系统预装（Win11 自带）                          | Tauri 运行时依赖               |

后端编译需要 MSVC（Microsoft C++ Build Tools，含 Windows SDK）。建议使用 VS Code 并安装 `rust-analyzer`、`Volar`（Vue 官方扩展）。

### 获取代码

仓库使用 git submodule（`resources/`），克隆时请一并拉取：

```bash
git clone --recurse-submodules git@github.com:Logical-Byte/open-endfield-assistant.git
# 如果已经克隆但没有子模块：
git submodule update --init --recursive
```

### 安装依赖

```bash
pnpm install
```

### 下载 OCR 模型

`models/` 目录已被 git 忽略（体积较大，不入库），克隆仓库后需要先下载 OCR 模型才能正常识别档案标题：

```bash
pnpm download:models
```

已存在的文件会跳过，需要强制覆盖时加 `--force`（`pnpm download:models --force`）。下载源为 ModelScope 的 `RapidAI/RapidOCR` 仓库，脚本位于 `scripts/downloadModels.ts`。

## 如何参与开发

1. 在 [GitHub](https://github.com/Logical-Byte/open-endfield-assistant) 上开 issue 说明要做什么，或认领已有 issue。
2. 从 `main` 分支切出功能分支：`git switch -c <your_name>/feat/xxx`，例如如果你的名字叫 `oceancat`，要做的功能是 `foo`，则分支名可为 `oceancat/feat/foo`。
3. 完成改动后，本地通过全部检查（见下文「本地调试」）。
4. 发起 Pull Request（PR），在描述中说明改动内容与验证情况。

### 代码规范（务必遵守）

- **依赖管理**：新增/移除依赖必须用 `pnpm add` / `cargo add`（除非必要不指定版本），不得手改 `package.json` / `Cargo.toml` 中的依赖项。
- **前端**：
  - 定义有名字的函数用 `function` 关键字，回调/匿名函数用箭头函数；
  - 本项目启用 TypeScript：所有函数参数与返回值都要有类型注解（返回 `void` 的除外）；
  - 需要编写组件时遵循 `nuxt-ui-expert` 规范；
  - 属性排序等可自动修复的问题，运行 `pnpm lint:fix` 即可。
- **后端**：
  - 使用 cargo 命令时注意工作目录（在 `src-tauri/` 下执行，或加 `--manifest-path src-tauri/Cargo.toml`）；
  - 原子引用计数的克隆必须写 `Arc::clone(&x)`，不能写 `x.clone()`；
  - 改完代码运行 `cargo check`；通过后运行 `cargo fix --allow-dirty` 与 `cargo clippy --fix --allow-dirty`，确保风格符合要求。
- **提交信息**：建议遵循 Conventional Commits（`feat:` / `fix:` / `refactor:` / `docs:` 等），保持简洁清晰。

## 本地调试

### 启动完整应用（推荐）

```bash
pnpm tauri dev
```

同时启动 Vite 开发服务器（`http://localhost:1420`，热更新）与 Tauri 调试窗口。前端改动即时生效；Rust 改动会触发重新编译。

### 只启动前端

```bash
pnpm dev   # http://localhost:1420（strictPort）
```

此时无法调用后端命令（`invoke` 会失败），仅适合纯 UI 调试。

### 后端日志

- 开发期：控制台输出 DEBUG+ 级别；
- 文件日志：`logs/YYYY-mm-dd.log`（TRACE+），按本地日期每日轮换，位于应用根目录（开发期为项目根）。

### 检查与测试

```bash
pnpm check        # typecheck + eslint + prettier 检查
pnpm lint:fix     # 自动修复可修复的 lint 问题
cargo test  # 后端单元测试（在 src-tauri/ 下执行）
```

### 数据生成

`resources/data/prts.json` 由解包数据生成，改了 `scripts/` 下的数据表脚本后需要重新生成：

```bash
pnpm makedata   # jiti scripts/makeAllData.ts
```

### 常用命令速查

| 命令                   | 作用                                          |
| ---------------------- | --------------------------------------------- |
| `pnpm install`         | 安装前端依赖                                  |
| `pnpm tauri dev`       | 启动开发环境（完整应用）                      |
| `pnpm dev`             | 仅前端 Vite 开发服务器                        |
| `pnpm check`           | 前端检查（typecheck + eslint + prettier）     |
| `pnpm lint:fix`        | 自动修复可修复的 lint 问题                    |
| `pnpm makedata`        | 重新生成 `resources/data/prts.json`           |
| `pnpm download:models` | 下载 OCR 模型到 `models/`（克隆仓库后先执行） |
| `cargo check`          | 后端编译检查（在 `src-tauri/` 下）            |
| `cargo test`           | 后端单元测试（在 `src-tauri/` 下）            |
| `pnpm package`         | 打包绿色便携 zip（见下）                      |

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
  2. 将 `target/release/oea.exe`（`--no-bundle` 下二进制沿用 Cargo 包名，需自行重命名）重命名为 `OEA.exe`，连同 `models/`、`resources/` 拷入暂存目录（跳过 `.` 开头的条目，如子模块 `.git`）；
  3. 用 Windows 自带 `tar.exe`（bsdtar）打 zip，并校验 zip 根目录包含 `OEA.exe`、`models/`、`resources/`。

产物：`releases/OEA-windows-x86_64-v0.1.0.zip`（约 39MB）。`releases/` 目录已 git 忽略。

### 冒烟测试

把 zip 解压到**干净目录**，双击 `OEA.exe`：

- 应用能正常启动；
- `logs/`、`cache/` 在 exe 旁自动生成（`cache/webview-data/` 为 WebView2 用户数据）；
- 扫描任务能正常加载 OCR 模型（`models/`）与 `resources/data/prts.json`。

## 发版流程

> **自动化发版**：推送 `v*` tag（如 `v0.1.0`）后，release workflow（`.github/workflows/release.yml`）会自动构建 zip 并发布到 GitHub Releases（自动生成 changelog）。
> **tag 必须与 `src-tauri/tauri.conf.json` 中的 `version` 一致**（例如版本为 `0.1.0` 时打 `v0.1.0`）。workflow 会在构建前校验二者一致，不一致则构建失败。

1. **同步版本号**（三处保持一致，例如要发 `0.1.0`）：
   - `package.json` 的 `version`
   - `src-tauri/tauri.conf.json` 的 `version`
   - `src-tauri/Cargo.toml` 的 `[package] version`
2. **确认资源完整**：检查 `resources/` 子模块内容（尤其未跟踪的 `data/`、`icons/`），必要时先提交到 `oea-resource` 仓库并更新子模块引用。
3. **本地全量检查**：`pnpm check` + `cargo check`（`src-tauri/` 下）+ `cargo test --lib`。
4. **打包**：`pnpm package`，得到 `releases/OEA-windows-x86_64-v0.1.0.zip`。
5. **冒烟测试**：见上文「冒烟测试」。
6. **打 tag 并推送**：

   ```bash
   git tag v0.1.0
   git push origin main --tags
   ```

   推送后 release workflow 会自动完成构建与 GitHub Release 发布（上传 zip、自动生成 changelog）。

7. **（可选）手动发布**：若 workflow 未自动创建 Release，可在 [Releases](https://github.com/Logical-Byte/open-endfield-assistant/releases) 页面以 `v0.1.0` 手动创建 release，上传 `releases/OEA-windows-x86_64-v0.1.0.zip`，填写变更说明。

## 自动化逻辑参考

以下内容记录自动化扫描所依赖的游戏界面特征与导航逻辑。修改 `src-tauri/src/scene/`、`src-tauri/src/tasks/archive_scan/` 相关代码时，请保持本文档同步更新。

### 坐标约定

本文档中，所有坐标均以 1280 × 720 为基准。如未特别说明，所有坐标均为 ltrb(x0, y0, x1, y1) 的形式，表示一个矩形区域的左上角坐标为 (x0, y0)，右下角坐标为 (x1, y1)。如果说明是 ltwh 坐标 (x0, y0, w, h)，则表示左上角坐标为 (x0, y0)，宽度为 w，高度为 h。

### 如何导航到档案库主界面

- 如果已经在档案库主界面则不用动
- 如果在协议终端界面，则点击档案库
- 如果在大世界界面，则先按 ESC 进入协议终端
- 如果在档案库子界面，则点击关闭按钮返回档案库主界面
- 如果在档案详情页面，则点击关闭按钮返回档案库子界面
- 如果在其他界面，则不受支持

### 每个界面的特征和跳转关系

- 档案库主界面和每个档案库子界面在 (0, 0, 162, 76) 范围内都有 “情报档案库/情报档案库标题”
- 档案库主界面：
  - 有 “情报档案库/情报档案库标题”，上面已经说过了
  - 在 (1180, 0, 1280, 100) 范围内有 “情报档案库/档案库主界面关闭” 按钮，点击它可以返回协议终端界面
  - 在 (692, 371, 959, 601) 范围内有 “情报档案库/音像存档” 按钮，点击它可以进入 “音像存档 - 多媒体” 子界面
  - 在 (957, 135, 1221, 371) 范围内有 “情报档案库/见闻辑录” 按钮，点击它可以进入 “见闻辑录 - 纸质记录” 子界面
  - 在 (958, 369, 1220, 601) 范围内有 “情报档案库/中枢档案” 按钮，点击它可以进入 “中枢档案 - 中枢档案” 子界面
- 档案库子界面：
  - 有 “情报档案库/情报档案库标题”，上面已经说过了
  - 音像存档相关的所有子界面在 (52, 482, 189, 618) 范围内有 “情报档案库/音像存档水印”
  - 见闻辑录相关的所有子界面在 (52, 482, 189, 618) 范围内有 “情报档案库/见闻辑录水印”
  - 中枢档案相关的所有子界面在 (52, 482, 189, 618) 范围内有 “情报档案库/中枢档案水印”
  - 确定在某个分类的子界面后，可以根据颜色来判断具体在哪个子界面。
    - 从上到下有 3 个 roi，分别是 ltwh(180, 120, 60, 36)、ltwh(180, 184, 60, 36)、ltwh(180, 248, 60, 36)，需判断这些区域的平均颜色是深色还是浅色，阈值为 128，深色表示当前在这个子界面，浅色表示不在这个子界面。
    - 例如，如果在 “音像存档” 分类的子界面，则不需要判断颜色，因为音像存档只有一个子界面。
    - 例如，如果在 “见闻辑录” 分类的子界面，则需要判断全部 3 个 roi，如果第 2 个 roi 是深色，说明当前在 “见闻辑录 - 电子档案” 子界面。
    - 例如，如果在 “中枢档案” 分类的子界面，则只需要判断前 2 个 roi，如果第 1 个 roi 是深色，说明当前在 “中枢档案 - 中枢档案” 子界面。
    - 点击对应的 roi 可以在同一分类的子界面之间切换。
    - 例如，如果在 “见闻辑录 - 纸质记录” 子界面，则点击第 2 个 roi 可以切换到 “见闻辑录 - 电子档案” 子界面，点击第 3 个 roi 可以切换到 “见闻辑录 - 藏品” 子界面。
    - 每个子界面在 (1180, 0, 1280, 100) 范围内都有 “档案库子界面关闭” 按钮，点击它可以返回档案库主界面，子界面的说明见 [档案库有哪些子界面](#档案库有哪些子界面)
- 档案详情页面的特征是：(356, 34, 496, 77) 范围内有 “档案详情装饰”，且 (1180, 0, 1280, 100) 范围内有 “情报档案库/档案详情关闭” 按钮，点击它可以返回档案库子界面（但是具体返回哪个子界面取决于当前档案详情页面属于哪个子界面）。
- 协议终端界面的特征是：(971, 108, 1280, 700) 范围内有 “档案库” 按钮，点击它可以进入档案库主界面
- 大世界界面的特征是：(1180, 0, 1280, 100) 范围内有 “协议终端” 按钮，按 ESC 可以进入协议终端界面

### 档案库有哪些子界面

档案库有主界面和多个子界面，子界面包括：

- 音像存档 - 多媒体
- 见闻辑录 - 纸质记录
- 见闻辑录 - 电子档案
- 见闻辑录 - 藏品
- 中枢档案 - 中枢档案
- 中枢档案 - 调查报告

从主界面点击音像存档后，会进入 “音像存档 - 多媒体” 子界面，从主界面点击见闻辑录后，会进入 “见闻辑录 - 纸质记录” 子界面，从主界面点击中枢档案后，会进入 “中枢档案 - 中枢档案” 子界面。

### 如何在档案库中导航

进入档案库主界面后，搜索并点击 “音像存档” 按钮，进入 “音像存档 - 多媒体” 子界面，然后点击 (401, 182)，这个坐标是第 1 份档案的坐标，点击它进入第 1 份档案的详情页面，然后根据 [如何扫描一个子分类中的所有档案](#如何扫描一个子分类中的所有档案) 中的描述，对音像存档 - 多媒体分类中的档案进行扫描。

扫描完成后，返回到音像存档 - 多媒体子界面，然后点击关闭按钮返回档案库主界面，然后在 (957, 135, 1221, 371) 范围内搜索并点击 “情报档案库/见闻辑录” 按钮，进入 “见闻辑录 - 纸质记录” 子界面，同样进入第 1 份档案的详情页面，用相同的方法扫描这个子分类中的所有档案，扫完回到 “见闻辑录 - 纸质记录” 子界面，然后进入 “见闻辑录 - 电子档案” 子界面，扫描完后进入 “见闻辑录 - 藏品” 子界面，扫描完后返回档案库主界面，然后进入 “中枢档案 - 中枢档案” 子界面，扫描完后进入 “中枢档案 - 调查报告” 子界面，扫描完后返回档案库主界面。

这样，就扫描完了全部 6 个子分类中的所有档案。结束。

### 如何扫描一个子分类中的所有档案

进入一个子分类中的第 1 份档案的档案详情页面后，需要在 (350, 58, 578, 42) 范围内进行 ocr，把结果写到日志，级别为 SUCCESS。
识别完成后，在 (762, 654, 925, 711) 范围内搜索 “下一篇” 按钮，如果找到了就点击它进入下一份档案的详情页面，并继续扫描。
如果没找到 “下一篇” 按钮，则在 (1206, 313, 1276, 423) 范围内搜索 “情报档案库/档案详情右箭头” 按钮，如果找到了就点击它进入下一份档案的详情页面，并继续扫描。
如果两个按钮都没找到，则说明已经扫描完了这个子分类中的所有档案，点击关闭按钮返回档案库子界面，然后导航到下一个子界面并继续。

## 关于解包数据

一级子分类：`PrtsPage.json`
二级子分类：`PrtsCategory.json`

档案库子界面的档案标题和档案详情页面的档案标题不一致，以下列出差异

### 音像存档 - 多媒体

| Table                     | 1                     | 2          |
| ------------------------- | --------------------- | ---------- |
| `PrtsAllItem.json`        | 仇恨书写者的录音·其一 | 医生的留声 |
| `PrtsFirstLv.json`        | 仇恨书写者的录音      | 医师的留声 |
| **`PrtsMultimedia.json`** | 仇恨书写者的录音·其一 | 医生的留声 |
| `ReadingPopUpTable.json`  | 仇恨书写者的录音·其一 | 医师的留声 |

### 见闻辑录

| Table                       | 1                      | 2              | 3                                | 4                |
| --------------------------- | ---------------------- | -------------- | -------------------------------- | ---------------- |
| `PrtsAllItem.json`          | 被记录的碾骨恶行（一） | 被扯碎的手写信 | 一张在不同铁笼间传递的纸条（一） | 哈特曼记录·其一  |
| `PrtsFirstLv.json`          | 被记录的碾骨恶行       | 被扯碎的手写信 | 在不同铁笼间传递的纸条           | 哈特曼记录-其一  |
| `PrtsRecord.json`           | 被记录的碾骨恶行（一） | 被扯碎的手写信 | 一张在不同铁笼间传递的纸条（一） | 哈特曼记录·其一  |
| **`RichContentTable.json`** | 被记录的碾骨恶行（一） | 被撕碎的手写信 | 一张在不同铁笼间传递的纸条（一） | 哈特曼记录：其一 |

### 中枢档案

| Table                       | 1                      |
| --------------------------- | ---------------------- |
| `PrtsAllItem.json`          | 材料研究所实验报告留档 |
| `PrtsDocument.json`         | 材料研究所实验报告留档 |
| `PrtsFirstLv.json`          | “打潮鞭”项目调查报告   |
| **`RichContentTable.json`** | 材料研究所实验报告留档 |
