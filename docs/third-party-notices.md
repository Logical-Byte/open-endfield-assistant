# 第三方组件

## EER 基质规则数据

`resources/data/essence_catalog.json` 包含基质属性名称、类型以及武器属性组合，来源为
[eer-resource](https://github.com/Logical-Byte/eer-resource) 的
`f10cada35e2569c248a89df5b2b59552b37876fc`，通过 `scripts/importEssenceCatalog.py` 提取。

扫描和分类设计参考 [EER](https://github.com/Logical-Byte/endfield-essence-recognizer)
`066d0e5b830fa06e0addbb52966fe6513b7e4eb1`。OEA 只实现独立分类所需的武器匹配、
自定义组合、高等级保留规则，不包含数量认领与冗余清理。EER 仓库声明使用 AGPL-3.0，
本页记录参考来源，不替代上游许可文本。

`resources/templates/基质/` 的 36 个模板由该 EER 版本中的属性文字模板和五个界面状态
截图通过 `scripts/importEssenceTemplates.py` 重采样至 720p。目录内 `LICENSE.EER`
保存上游的许可文本。模板与目录数据的更新方法见 [基质扫描](essence-automation.md)。

## RapidOCR OCR 模型

OEA 使用以下 OCR 模型文件：

- `resources/ocr-models/PP-OCRv6_rec_tiny.onnx`
- `resources/ocr-models/ppocrv6_tiny_dict.txt`

来源：

- [RapidAI/RapidOCR 模型仓库（ModelScope）](https://www.modelscope.cn/models/RapidAI/RapidOCR)
- [RapidAI/RapidOCR 项目（GitHub）](https://github.com/RapidAI/RapidOCR)

本页仅记录第三方文件的来源，不对上游未随模型仓库提供的许可证作补充或推断。
