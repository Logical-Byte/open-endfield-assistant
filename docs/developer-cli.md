# 开发者 CLI

Developer CLI 是独立的开发者工具，通过 `oea-cli` 调用共享后端 library，不启动桌面应用。命令、状态名和 JSON schema 随内部实现调整，不承诺兼容性。

在仓库根目录显式选择 `oea-cli` package：

```bash
cargo run -p oea-cli -- --help
cargo run -p oea-cli -- match-image --image screen.png --template template.png --region 100,50,320,180 --json
```

在 Windows 仓库中调试窗口命令时，导航命令从仓库根目录读取模板资源。添加 `--release` 只改变 Cargo 优化等级，不改变资源来源：

```powershell
cargo run -p oea-cli -- connect --json
cargo run -p oea-cli -- navigate archive-main
cargo run -p oea-cli -- screenshot -o captures --crop 100,50,320,180 --json
cargo run -p oea-cli -- match --template candidate.png --region 100,50,320,180 --json
```

`captures` 必须预先存在。`cargo build -p oea-cli --release` 未启用 `tauri/custom-protocol`，仍使用工作区资源，且仅编译程序，不复制资源。导航模板的资源根目录由应用形态决定，OEA release 使用可执行文件所在目录，分发导航命令时需要在程序旁准备 `resources/templates`。`ocr` 通过 `--models` 显式选择模型目录。`connect`、`screenshot`、`match` 和 `match-image` 无需应用的 OCR 模型或模板资源。

使用 `cargo build -p oea-cli` 生成 `target/debug/oea-cli.exe`。CLI 始终使用 Console subsystem，GUI 的 release profile 构建使用 GUI subsystem。CLI 版本继续读取 `crates/oea/tauri.conf.json`。CLI 的通用图片识别直接使用 `oea-vision`，游戏连接、导航和模板资源加载使用 `oea` 公开的模块，不需要 `cli` feature，目前仍传递依赖 Tauri 和 GUI 构建依赖。CLI 仅提供开发构建，不进入发行包或发布附件。

窗口命令在 Windows 中复用生产连接检查和坐标转换，需要真实游戏窗口。CLI 按命令装配截图器和导航能力，不创建 `Session`，窗口命令均不加载 OCR。仅 `navigate` 使用应用模板目录。`connect` 会恢复窗口并自动调整为 1280×720 游戏画面（Windows 客户端是客户区，云终末地是浏览器网页视口），保留该尺寸定位到屏幕内。全屏时先等待前台激活完成，Windows 客户端通过 Alt+Enter 退出，Chromium 云终末地通过携带扫描码的长按 Esc 退出。连接过程不截图、不点击游戏界面。`navigate` 等待生产导航器完成目标状态或返回错误，没有额外 CLI timeout。macOS 和 Linux 的窗口命令返回 `platform_unsupported`。`match-image` 可跨平台运行，无需游戏或 OCR 模型。

```text
connect / navigate / screenshot / match
  game_environment::connect -> ResolutionTransform
  screenshot / match -> oea_window::Window::screenshot -> 1280×720 Lanczos3
  navigate -> CLI NavigationContext -> Navigator::navigate_to
    NavigationContext 组合截图、输入、模板缓存和时钟，沿用生产输入的等待与鼠标归位规则

match-image
  本地原图，不缩放 -> vision::template::match_
match
  生产 720p 图像 -> vision::template::match_
```

`--crop` 和 `--region` 使用 `LEFT,TOP,WIDTH,HEIGHT`，四个字段均为十进制无符号整数。宽高必须为正，整个矩形必须位于输入图像内。窗口命令使用 1280×720 坐标，本地图像使用原始像素坐标。匹配返回最高分位置和原始 `CCOEFF_NORMED` score，不添加 threshold。

输入图片、模板和截图输出的相对路径均基于当前工作目录。截图省略 `-o` 时保存到当前目录，指定已有目录时生成 `oea-screenshot-YYYYMMDD-HHMMSS-mmm.png`。指定新文件时父目录必须存在，文件名允许 `.png` 或无扩展名。所有截图始终编码为 PNG，已有文件和自动命名碰撞均报错，不覆盖或自动创建目录。结果中的路径为绝对路径。输出路径中的非 Unicode 字符使用替换字符显示，文件操作仍使用原始路径。此类输出路径仅供展示，不能保证用于重新打开文件。

`--json` 可出现在 `oea-cli` 后的正常 flag 位置。成功结果只写 stdout。运行失败返回退出码 1，错误写 stderr，JSON 模式写单个错误对象。如执行期间产生 warning，JSON 失败结果将诊断文本纳入 `error.message`，保证 stderr 仍可直接解析为一个 JSON 对象。成功或 human 模式的诊断使用 stderr 文本。参数错误使用 clap 文本与退出码 2。只输入 `oea-cli` 显示帮助并返回 2，各级 `--help` 和 `--version` 返回 0。

验证命令：

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Windows 真机验收应覆盖 1280×720 和更高的 16:9 客户区尺寸，比较生产匹配分数与 CLI 结果，检查截图元数据、导航完成时机以及 release 的输出流和退出码。macOS 可以运行本地图像测试，并通过 `cargo xwin clippy --workspace --target x86_64-pc-windows-msvc --all-targets --all-features -- -D warnings` 检查 Windows 编译。真实窗口行为仍需 Windows 交互桌面验收。
