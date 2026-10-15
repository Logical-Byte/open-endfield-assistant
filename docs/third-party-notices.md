# 第三方组件

## RapidOCR OCR 模型

OEA 使用以下 OCR 模型文件：

- `resources/ocr-models/PP-OCRv6_rec_tiny.onnx`
- `resources/ocr-models/ppocrv6_tiny_dict.txt`

来源：

- [RapidAI/RapidOCR 模型仓库（ModelScope）](https://www.modelscope.cn/models/RapidAI/RapidOCR)
- [RapidAI/RapidOCR 项目（GitHub）](https://github.com/RapidAI/RapidOCR)

本页仅记录第三方文件的来源，不对上游未随模型仓库提供的许可证作补充或推断。

## 游戏展示图标

`resources/assets/itemicon/` 中的 PNG 来自 [EER](https://github.com/Logical-Byte/endfield-essence-recognizer) 随应用提供的游戏图标，参考 commit 为 `066d0e5b830fa06e0addbb52966fe6513b7e4eb1`。游戏数据表来自 [AKEData 数据服务](https://data.akedata.wiki/manifest.json)，本次生成使用 `1.5.3@10731330-8`。

## 基质识别参考与模板

`automation::essence_scan` 参考同一 EER commit 的布局、词条、等级、稀有度与标记识别。
`resources/templates/基质/` 中的 36 张 PNG 来自其 `templates/generated/` 和
`templates/screenshot/`，按 `2/3` 缩放到 720p。坐标与识别常量在对应 Rust 文件中注明来源。
EER 仓库提供 GNU AGPL v3 许可证，保留在模板目录的 `LICENSE.EER`。
