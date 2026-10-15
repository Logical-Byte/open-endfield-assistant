# EER 集成的数据依赖调查

调查日期：2026-10-14。

检查本地 EER 实现 `/Users/chen/Workspace/repos/endfield-essence-recognizer`，commit `066d0e5b830fa06e0addbb52966fe6513b7e4eb1`，并下载解包原表实测。下面先列结论，再列实现依据。KiB/MiB 使用 1024 进制，文件大小均为未压缩文件字节数，不是进程内存。

## 集成成本结论

- **核心基质能力：生成阶段新增原表 0 张。** `scripts/gameData.ts` 已在模块初始化时读取 `GemTable`、`GemTagIdTable`、`WeaponBasicTable`、`SkillPatchTable`、`ItemTable` 和中文文本表。核心新增工作是投影这些已读数据，输出词条目录及武器词条组合。现有生成任务没有使用或输出基质数据。
- **完整沿用 EER 五份投影：新增原表 4 张，共 443,387 B（433.00 KiB）。** 另外 9 张已由 OEA 加载。四张新增表见下文。13 张依赖表总大小 22,150,248 B（21.12 MiB），不能全部算成增量。
- **核心运行时业务 JSON：26,615 B（25.99 KiB）。** 这是完整 `EssenceStat.json` 和 `Weapon.json` 的大小，已经包含武器展示字段，可视为沿用 EER schema 的保守预算。
- **五份业务 JSON 全部打包：37,849 B（36.96 KiB）。** 与当前 OEA 两份档案 JSON 共 361,343 B 相比，静态数据文件增加约 10.47%。这是文件体积比较，不代表运行时内存增加比例。
- **复用 EER 的识别方案：另加 38 张模板，共 43,151 B（42.14 KiB）。** 核心两份 JSON 加模板为 69,766 B（68.13 KiB）。五份 JSON 加模板为 81,000 B（79.10 KiB）。不需要新增神经网络模型。
- **展示图可选：** EER 本地 105 张展示 PNG 共 2,362,838 B（2.25 MiB）。全部五份 JSON、核心模板、现有展示图合计 2,443,838 B（2.33 MiB），不含代码、依赖库、运行时用户扫描结果及远程加载的副本背景图。

OEA 原表加载证据：[gameData.ts](../../scripts/gameData.ts)。现有输出仅包含档案文件，见 [makeAllData.ts](../../scripts/makeAllData.ts)。Rust 运行时也只读取 `prts.json` 和 `archive_contract.json`，见 [database.rs](../../src-tauri/src/data/archive/database.rs)。因此“生成脚本已读原表”和“应用运行时已有可用基质数据”必须分开看。

新增四张原表：

| 原表                 | 实测字节数 | 用途                       |
| -------------------- | ---------: | -------------------------- |
| `WikiEntryDataTable` |    378,379 | 图鉴条目映射回武器 item ID |
| `WikiEntryTable`     |     52,611 | 武器分类中的条目列表       |
| `WikiGroupTable`     |     12,077 | 武器分类名称、图标、排序   |
| `RarityColorTable`   |        320 | 界面配色                   |

三个 Wiki 表仅服务 EER 的武器分类生成方式。实测 `WeaponBasicTable.weaponType` 到五个分类的对应关系为 `1 → SWORD`、`2 → WAND`、`3 → CLAYM`、`5 → LANCE`、`6 → PISTOL`。若 OEA 使用稳定的类型映射及自己的 UI 配色，四张新增表都可以省略。是否采用这条路径属于实现选择。

## 解包数据及测量记录

用户提供的 [asset-sync-index.json](https://data.akedata.wiki/asset-sync-index.json) 的 revision 是 `2026-10-12T17:49:18.192150+00:00`。该索引只有 `json`（13,425 文件，677,026,382 B）和 `images`（33,137 文件，34,749,860,506 B）两个 dataset，没有 `TableCfg`。基质依赖不要求读取这些全部 Json 或图片。

TableCfg 版本由独立的 [manifest.json](https://data.akedata.wiki/manifest.json) 指定。本次固定使用其 latest `1.5.3@10731330-8`，下载地址为 `https://data.akedata.wiki/public/1.5.3/10731330-8/TableCfg/<表名>.json`。两套版本彼此独立，见数据站点的 [DEVELOPMENT.md](https://github.com/NagiYume/AKEDatabase/blob/main/DEVELOPMENT.md)。

缓存位于 `/Users/chen/Workspace/repos/akedata-cache/`：索引、manifest、13 张依赖表、每表 URL/行数/字节数/SHA-256 的 `table-stats.json`，以及实测生成的 `eer-generated/`。这是本次调查的部分缓存，未下载 OEA 完整构建需要的所有输入，不能直接作为完整 `ENDFIELD_DATA_DIR` 使用。

实际运行 EER 的 `scripts/transform_weapon_data.py` 后，得到 80 武器、31 词条（5 ATTRIBUTE、12 SECONDARY、14 SKILL）、5 武器类型、12 刷取地点和 6 配色。五份 JSON 的解析内容与 EER 已携带的资源全部一致。5 把三星武器的 `stat2_id` 为 null，这是原投影的合法结果，不能假定所有武器都有三个非空词条。

| 生成文件               | 条目数 | 字节数 |
| ---------------------- | -----: | -----: |
| `Weapon.json`          |     80 | 22,836 |
| `EssenceStat.json`     |     31 |  3,779 |
| `WeaponType.json`      |      5 |    958 |
| `EnergyAlluviums.json` |     12 |  9,950 |
| `RarityColor.json`     |      6 |    326 |

图片索引需要按完整目录选取，不能只按图标 basename 搜索。80 个武器 icon ID 在索引里匹配到 249 文件，分布于抽卡大图、不同尺寸图标和重复 UI prefab。EER 实际使用 `sprites/itemicon/`，该目录的 80 张图合计 5,261,280 B（5.02 MiB）。同一批 `itemiconbig/` 为 21,008,320 B（20.04 MiB），`gachaweapon/` 为 671,606,080 B（640.49 MiB）。武器分类目录 `sprites/wiki/groupicon/` 的五张原图另计 54,680 B。这些为索引标注的原图大小，未下载原图，不等于 EER 本地优化后 PNG 的体积。

## 推荐管线范围

沿用 OEA 已有的生成、Rust 投影、IPC 契约流程，新增独立的基质数据领域。首期只投影词条 ID/名称/类型，以及武器 ID/名称/稀有度/三词条组合。武器分类、图标和刷取地点按界面需求逐步增加。后台识别和规则评估不需要加载 11.36 MiB 中文全文表或 6.46 MiB 技能全文表，生成阶段取出目标字段即可。

以上文件体积可以实测。Rust HashMap、String、OpenCV 解码后的模板及前端查询索引会占额外内存，必须在最终加载结构确定后测量，不能把 JSON 字节数直接当作 RAM。

## 核心基质识别和规则评估

EER 运行时将五个预处理 JSON 一次加载到 `StaticGameData`，总计 37,849 B。见 [static_game_data.py:58](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/game_data/static_game_data.py:58)。为方便直接打开源码，下文使用本机绝对路径链接。

核心识别依赖 `EssenceStat`。当前快照有 31 条，3,779 B，只包含 `stat_id`、`name`、`type`。来源为 `GemTable` 的键、`tagName` 的中文翻译和 `termType`，0/1/2 分别对应 ATTRIBUTE/SECONDARY/SKILL。见 [transform_weapon_data.py:190](/Users/chen/Workspace/repos/endfield-essence-recognizer/scripts/transform_weapon_data.py:190)。

规则评估需要武器对应的三词条组合。当前 `Weapon` 有 80 条，22,836 B。关键字段是武器 ID、`stat1_id/stat2_id/stat3_id`，武器名用于展示。关联路径为 `WeaponBasicTable.weaponSkillList` → `SkillPatchTable.SkillPatchDataBundle[0].tagId` → `GemTagIdTable[tagId]` → `GemTable[statTermId].termType`。武器类型、稀有度和图标不参与基质组合匹配。见 [transform_weapon_data.py:269](/Users/chen/Workspace/repos/endfield-essence-recognizer/scripts/transform_weapon_data.py:269) 和 [classifier.py:338](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/scanner/classifier.py:338)。

单次识别返回三词条 ID、类型、等级、稀有度及锁定/弃用状态。31 个词条文字模板按 `stat_id.png` 命名，共 33,097 B。EER 用 `EssenceStat` 列表构建模板标签，OpenCV 执行模板匹配，阈值为 0.75/0.50。见 [attribute.py:30](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/attribute.py:30) 和 [template_recognizer.py:68](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/template_recognizer.py:68)。

等级由 ROI 内连续亮点数量判断，亮度阈值 200、采样半径 2。稀有度使用固定 BGR 颜色和色相阈值，没有读取 `RarityColorTable`。这些算法没有 OCR、ONNX 或其他神经网络模型资源。见 [attribute_level.py:27](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/attribute_level.py:27)、[essence_rarity.py:18](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/essence_rarity.py:18)、[engine.py:176](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/scanner/engine.py:176)。

## 自动扫描、滚动和标记

基质扫描额外使用 7 个 screenshot 模板，共 10,054 B：`武器基质.png`、`已锁定.png`、`未锁定.png`、`已弃用.png`、`未弃用.png`、`卡片锁定标记.png`、`卡片弃用标记.png`。与词条模板合计为 43,151 B。见 [ui.py:24](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/ui.py:24)、[abandon_lock_status.py:29](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/abandon_lock_status.py:29)、[skip_marker.py:95](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/recognition/tasks/skip_marker.py:95)。

1080p 基准 UI 坐标、词条 ROI、等级亮点、卡片网格和滚动条检查点均是源码常量。动态布局模块进行锚定/缩放。翻页和滚动条检测使用亮度与间隙检测，没有新增游戏表依赖。见 [res_1080p.py:14](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/layout/res_1080p.py:14)、[dynamic.py:48](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/layout/dynamic.py:48)、[engine.py:1617](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/scanner/engine.py:1617)。

`templates/screenshot` 还有 `运送委托列表_已激活.png` 和 `抢单报酬_武陵调度券.png`，共 17,084 B，属于委托抢单功能，不在基质核心集成范围。

## 设置、结果展示和可选刷取建议

`WeaponType` 快照有 5 条，958 B，包含 `weapon_type_id/wiki_group_id/name/icon_id/sort_order`。EER 使用 WikiGroup → WikiEntry → WikiEntryData.refItemId 关联武器分类。如果 OEA 已有武器分类投影，可直接复用其类型与展示数据。见 [transform_weapon_data.py:163](/Users/chen/Workspace/repos/endfield-essence-recognizer/scripts/transform_weapon_data.py:163)。

`RarityColor` 有 6 条，326 B，仅用于 UI 展示颜色。与识别算法的固定稀有度颜色是两个独立用途。见 [static_data_service.py:144](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/services/static_data_service.py:144)。

`EnergyAlluviums` 有 12 条，9,950 B，字段为 `battleId/battleName/imageUrl/secondaryStats/skillStats`。刷取地点来源为 `WorldEnergyPointGroupTable.secAttrTermIds/skillTermIds`。脚本从 `worldLevel2GameMechanicsIdMap` 取最高世界等级，再查 `WorldEnergyPointTable.gameName/levelId`。`LevelLoadingTable.bgNameGroup[0]` 只用于构建背景图片 URL。基质识别、规则评估和扫描不需要这些地点数据。见 [transform_weapon_data.py:314](/Users/chen/Workspace/repos/endfield-essence-recognizer/scripts/transform_weapon_data.py:314)。

EER 本地展示图片有 105 个 PNG，共 2,362,838 B。其中武器图 80 个/2,070,866 B、武器类别图 5 个/8,541 B、基质稀有度底图 4 个/62,075 B。其余包括技能、默认基质和默认武器图。核心扫描不读取这批展示图。14 个技能图标 ID 的映射写死在服务层，另使用 `item_gem_rarity_5` 和 `icon_wpngem_00`。见 [static_data_service.py:198](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/services/static_data_service.py:198)。

规则比较和刷取建议的概率、冷却脂权重也是源码常量。词条 1&2 升级成功率为 0.6/0.24/0.109/0.05/0.027，技能为 0.109/0.042。冷却脂阈值分别为 30/60/120/250/450 和 120/300，每次失败获得 10，每轮完美基质 3 个。当前 EER 没有为这些常量加载额外游戏表。见 [evaluate.py:16](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/scanner/evaluate.py:16) 和 [farming_calculator.py:28](/Users/chen/Workspace/repos/endfield-essence-recognizer/src/endfield_essence_recognizer/core/farming_calculator.py:28)。

## EER 构建脚本的原表输入

完整脚本一次性加载 13 张原表：`GemTable`、`GemTagIdTable`、`ItemTable`、`LevelLoadingTable`、`RarityColorTable`、`SkillPatchTable`、`WeaponBasicTable`、`WikiEntryDataTable`、`WikiEntryTable`、`WikiGroupTable`、`WorldEnergyPointGroupTable`、`WorldEnergyPointTable`、`I18nTextTable_CN`。这是完整五个投影的一揽子输入，不代表核心基质功能必须在 OEA 新增 13 张原表。见 [transform_weapon_data.py:107](/Users/chen/Workspace/repos/endfield-essence-recognizer/scripts/transform_weapon_data.py:107)。

与 OEA 对照后，确认这些原表已在生成阶段加载，但当前档案运行时投影没有武器或基质领域。原表下载量与最终运行时投影量已在本文开头分别计算。
