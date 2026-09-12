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

- `resources/data/prts.json`：档案层级、顺序、名称和标题。
- `resources/data/archive_contract.json`：档案分类、图标、获取方式及参数。

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
