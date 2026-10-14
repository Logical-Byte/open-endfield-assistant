# 档案匹配决策

领域含义见 [术语表](GLOSSARY.md)。后端建议的判定条件见 [MatchResult](../src-tauri/src/automation/archive_scan/matching.rs)，本文仅保留决策理由与来源。

## 决策理由

- 每次档案详情页面只展示一个具体档案，因此每条扫描记录最多匹配一个档案。同标题无法证明是同一份档案，不能让一次观察贡献多个档案的收集证据。
- 只对唯一精确命中自动建立匹配。模糊候选即使只有一个，也可能只是目录中唯一足够相近的标题，不能据此认定已经收集。
- 标题规范化、已知标题解释和模糊推荐集中在后端，使领域判断拥有同一事实来源。前端负责用户选择，保留原始观察与后端建议，以便区分自动判断和人工决定。
- 明确的人工未选择是一项决定，需要与尚未作出人工决定区分。重新计算建议不能悄悄恢复用户已清除的匹配。
- 参考正文用于帮助人工区分同名档案。采用人工白名单和可选首行，控制资源体积及 diff。本轮不加入正文 OCR，未来可以讨论正文证据如何改善自动匹配。

## 参考正文来源

两个「挂在竹子上的字条」具有相同标题和图标，正文可以提供人工区分依据。关联经 `PrtsAllItem.contentId → RichContentTable.contentList → I18nTextTable_CN/EN` 核实：

- `nar_paper_map02_93_1` 经 `text_sm2l3m1_1` 关联正文。中文首行为「先祖保佑我家那口子」，英文首行为「Protect my family」。
- `nar_paper_map02_93_2` 经 `text_sm2l3m1_2` 关联正文。中文首行为「保佑寨子■■■■」，英文首行为「Protect the ■■■■ Stockade」。遮罩共四个字符，按源文本保留。

核实使用 [akedata 原表](https://data.akedata.wiki/manifest.json) 的 `1.5.3@10731330-8` 快照。实际生成方式与类型传递见 [数据管线](data-pipeline.md)。

## 原型记录

已确认的临时原型保存于本地分支 `van/prototype/archive-matching`，提交 `c75ffa612d5267434315191751c5d95deadeb9e7`。原型回答了推荐候选与人工选择如何呈现的问题，正式界面复用评审后的下拉交互与 `ArchiveLabel`。
