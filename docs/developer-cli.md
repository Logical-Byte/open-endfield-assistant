# 开发者 CLI

Developer CLI 是默认关闭的调试入口。命令、状态名和 JSON schema 随内部实现调整，不承诺兼容性。

在 `src-tauri/` 中启用 `cli` feature：

```bash
cargo run --features cli -- dev --help
cargo run --features cli -- dev match-image --image screen.png --template template.png --region 100,50,320,180 --json
```

在 Windows 仓库中调试窗口命令时，使用 debug 构建。应用从仓库根目录读取 OCR 模型和模板资源：

```powershell
cargo run --features cli -- dev connect --json
cargo run --features cli -- dev navigate archive-main
cargo run --features cli -- dev screenshot -o captures --crop 100,50,320,180 --json
cargo run --features cli -- dev match --template candidate.png --region 100,50,320,180 --json
```

`captures` 必须预先存在。release 构建从可执行文件所在目录读取资源。`cargo build --release --features cli` 仅编译程序，不复制资源。运行窗口命令前，需要按应用分发目录布局在可执行文件旁准备 `resources/ocr-models` 和 `resources/templates`。本地 `match-image` 使用显式文件路径，无需这些资源。

Windows 启用 `cli` 的 release 构建使用 Console subsystem。正式 release 不启用 `cli`，仍使用 GUI subsystem。启用 `cli` 后以普通模式启动 Tauri 时可能附带控制台。

窗口命令在 Windows 中复用生产流程，需要真实游戏窗口和应用的 OCR 模型、模板资源。`connect` 会恢复最小化窗口并确保窗口位于屏幕内，不截图、不发送点击或按键。`navigate` 等待生产导航器完成目标状态或返回错误，没有额外 CLI timeout。macOS 和 Linux 的窗口命令返回 `platform_unsupported`。`match-image` 可跨平台运行，无需游戏或 OCR 模型。

```text
connect / navigate / screenshot / match
  DPI 初始化 -> Session::connect
  screenshot / match -> Session::screenshot -> 1280×720 Lanczos3
  navigate -> Navigator::navigate_to -> 生产识别、输入、等待与重试

match-image
  本地原图，不缩放 -> template_matching::pure::match_template_in_region
match
  生产 720p 图像 -> template_matching::pure::match_template_in_region
```

`--crop` 和 `--region` 使用 `LEFT,TOP,WIDTH,HEIGHT`，四个字段均为十进制无符号整数。宽高必须为正，整个矩形必须位于输入图像内。窗口命令使用 1280×720 坐标，本地图像使用原始像素坐标。匹配返回最高分位置和原始 `CCOEFF_NORMED` score，不添加 threshold。

输入图片、模板和截图输出的相对路径均基于当前工作目录。截图省略 `-o` 时保存到当前目录，指定已有目录时生成 `oea-screenshot-YYYYMMDD-HHMMSS-mmm.png`。指定新文件时父目录必须存在，文件名允许 `.png` 或无扩展名。所有截图始终编码为 PNG，已有文件和自动命名碰撞均报错，不覆盖或自动创建目录。结果中的路径为绝对路径。输出路径中的非 Unicode 字符使用替换字符显示，文件操作仍使用原始路径。此类输出路径仅供展示，不能保证用于重新打开文件。

`--json` 可出现在 `dev` 后的正常 flag 位置。成功结果只写 stdout。运行失败返回退出码 1，错误写 stderr，JSON 模式写单个错误对象。如执行期间产生 warning，JSON 失败结果将诊断文本纳入 `error.message`，保证 stderr 仍可直接解析为一个 JSON 对象。成功或 human 模式的诊断使用 stderr 文本。参数错误使用 clap 文本与退出码 2。只输入 `dev` 显示帮助并返回 2，各级 `--help` 和 `--version` 返回 0。

验证命令：

```bash
cargo test --features cli
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Windows 真机验收应覆盖 1280×720 和更高的 16:9 客户区尺寸，比较生产匹配分数与 CLI 结果，检查截图元数据、导航完成时机以及 release 的输出流和退出码。macOS 可以运行本地图像测试，并通过 `cargo xwin clippy --target x86_64-pc-windows-msvc --all-targets --all-features -- -D warnings` 检查 Windows 编译。真实窗口行为仍需 Windows 交互桌面验收。
