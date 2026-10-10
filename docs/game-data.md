# 解包数据参考

## 数据生成

`pnpm makedata` 执行 [scripts/makeAllData.ts](../scripts/makeAllData.ts)，将游戏解包数据转换为 OEA 使用的档案数据。更新解包数据或修改生成脚本后，在仓库根目录运行：

```bash
pnpm makedata
```

### 输入

通过环境变量 `ENDFIELD_DATA_DIR` 指定解包数据根目录，也可在仓库根目录的 `.env` 中设置。脚本读取该目录下的：

- `TableCfg/`：游戏数据表和多语言文本。
- `Json/GameplayConfig/`：NPC 与世界实体配置。
- `Json/MissionRuntimeAsset/`：任务与子任务数据。
- `Json/LevelData/`：关卡、地图点位与交互数据。
- `Json/LevelScriptData/`：关卡脚本数据。

### 输出

命令生成并覆盖：

- `resources/data/prts.json`：共享档案层级与顺序，名称和标题字段包含 `zh-CN`、`en-US`。
- `resources/data/archive_contract.json`：档案分类、图标、获取方式及参数。

## 双语文本完整性

档案生成只读取必需档案表与 CN/EN 文本表，不依赖其他语言。TranslationKey 的整数 ID 在 JSON 解析时从原始 token 转为精确字符串，超过 JavaScript 安全整数范围也不舍入。它只用于生成时关联译文，不进入运行时目录。

两种译文均必需。源语言表缺失、非零键缺译、空白译文及零 ID 空引用分别报错，字段错误包含语言、实体、字段与 TranslationKey。源 `text` 和另一种语言不承担回退。

音像存档标题使用名称。文本与文档使用 RichContentTable 的详情标题。仅富文本行缺失时，双语标题共同回退名称并记录档案 ID。行存在时的缺译或空白仍使生成失败。层级过滤、排序和标题来源只决定一次，两种译文填入同一结构。

资源更新先完成生成及真实 Database 加载验证，直接提交并推送 resources 仓库，确认远端 commit 可取得后，再将消费者 schema 和 submodule pin 一起提交主仓库。获取契约继续按 ArchiveId 关联，不按语言重复生成。

## 数据表与标题差异

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
