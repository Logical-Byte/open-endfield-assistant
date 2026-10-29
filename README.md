# OEA：终末地小助手

[![Rust](https://img.shields.io/badge/Rust-1.99+-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-purple.svg?logo=tauri&logoColor=white)](https://tauri.app/)
[![Vue.js](https://img.shields.io/badge/Vue.js-3.x-green.svg?logo=vue.js)](https://vuejs.org/)
[![Nuxt UI](https://img.shields.io/badge/Nuxt%20UI-4.x-lightblue.svg?logo=nuxtdotjs&logoColor=white)](https://ui.nuxt.com/)
[![反馈交流群](https://img.shields.io/badge/反馈交流群-954628501-orange.svg?logo=qq)](https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms)
[![官网](https://img.shields.io/badge/官网-终末地一图流-yellow.svg)](https://ef.yituliu.cn/resources/oea)

> 想参与开发？请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

- [OEA 官网（下载最新版）](https://ef.yituliu.cn/resources/oea)
- [反馈交流群：954628501](https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms)

OEA 是开源免费的终末地小助手，支持档案查漏补缺和百趣演武自动抽。

- **档案查漏补缺**：一键识别终末地档案库，并同步到 OEM（终末地地图集），助力每一个全收集梦想！
- **百趣演武自动抽**：抽到想要的牌型为止，解放双手，摆完挂机！

![OEA-image-3](https://cos.yituliu.cn/endfield/oea/assets/oea-image-3.png)
![OEA-image-2](https://cos.yituliu.cn/endfield/oea/assets/oea-image-2.webp)
![OEA-image-4](https://cos.yituliu.cn/endfield/oea/assets/oea-image-4.png)

## 使用前准备

1. 前往 [OEA 官网](https://ef.yituliu.cn/resources/oea) 下载，**解压**后运行 `OEA.exe`。
2. 打开终末地，设置为 **1280 × 720 窗口模式**、**简体中文**。
3. **关闭 HDR**，关闭性能监控软件和任何会遮挡终末地窗口的软件。

理论上支持任意 **16:9** 的分辨率，我们最建议使用 **1280 × 720 窗口模式**，这个分辨率可以兼顾准确性和性能。

## 档案查漏补缺

1. 终末地打开**档案库主界面**。
2. 点击**开始扫描**，或按 `'`（引号键）。
3. 扫完点击**导出收集状态**。

理论上支持从协议终端、大世界和任意档案库界面开始扫描。为了稳定性，建议始终从**档案库主界面**开始扫描。

## 百趣演武自动抽

1. 终末地打开**百趣演武的抽牌界面**。
2. 选择**牌型目标**。
3. 点击**开始抽卡**，或按 `;`（分号键）。
4. 满足**任意一个**目标后会自动停止抽卡。

## 快捷键

- 按 `'`（引号键）开始扫描档案库，扫描过程中再次按下可停止。
- 按 `;`（分号键）开始百趣演武自动抽牌，再次按下可停止。
- 按 `Alt` + `Delete` 退出程序。

## 常见问题

- **手机能用吗？**

  不能。OEA 仅支持 Windows 10 / 11（x86_64）。

- **识别结果不准确怎么办？**

  可以使用输入框进行人工纠错。建议将识别错误告知我们，以便改进识别算法。

- **OEA 收费吗？**

  OEA 开源且免费，不会以任何形式收取费用。如果您是通过付费方式获取的 OEA，您可能已经被不法商家欺骗，请立即告知我们。

- **OEA 和 Mirror酱的关系是什么？**

  [Mirror酱](https://mirrorchyan.com/) 是独立的第三方应用分发平台，提供加速下载服务，需要付费使用。

  OEA 本身不收取任何费用，也提供免费的下载渠道。您可以前往 [GitHub Releases](https://github.com/Logical-Byte/open-endfield-assistant/releases) 免费下载和使用。

## 反馈交流

- [反馈交流群：954628501](https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms)
- [提交 GitHub Issue](https://github.com/Logical-Byte/open-endfield-assistant/issues)

遇到问题或建议，欢迎反馈并附上应用目录下 `logs/` 中的日志文件，便于定位问题。

## 致谢

- [终末地一图流](https://ef.yituliu.cn/)
- [终末地地图集](https://oem.re/)
- [逻辑元LogicalByte](https://space.bilibili.com/688411531)
- [Mirror酱](https://mirrorchyan.com/)
- [RapidAI/RapidOCR](https://github.com/RapidAI/RapidOCR)
- [RapidOCR 模型](https://www.modelscope.cn/models/RapidAI/RapidOCR)（[第三方组件说明](docs/third-party-notices.md)）
- [MaaXYZ/MaaFramework](https://github.com/MaaXYZ/MaaFramework)
- [MistEO/MXU](https://github.com/MistEO/MXU)
- [MaaEnd/MaaEnd](https://github.com/MaaEnd/MaaEnd)

## 说明

1. 自动更新功能有删除硬盘上的文件的操作，请确保重要数据已备份再使用自动更新功能，避免误删重要文件。
2. 机器识别，可能存在错误。若发现错误，欢迎反馈。
3. 本工具按 “原样”、“包含全部错误” 和 “视可用性情况” 提供，作者不对可用性、准确性或使用效果做出任何承诺或保证。
4. 使用者必须确保使用本工具符合相关法律法规与服务条款，禁止用于任何违法或侵权行为。
5. 使用者需承担因使用本工具产生的任何风险、损失或责任。
6. 使用本工具即意味着您同意以上全部内容。
