# 自更新 crate 拆分设计

状态：代码迁移已完成，Windows 实机更新验收仍待完成。

## 已确认的目标

将 OEA 自更新能力整理为 OEA 专用、脱离 Tauri 的更新内核，以便独立验证检查、下载和安装恢复。通用更新库与单纯目录搬迁不是本次选择的目标。

已确认的范围与约束：

- 首次拆分覆盖版本检查、下载源解析、下载校验、状态机、安装事务和启动恢复。
- 保持现有发布 ZIP、增量包格式、磁盘事务、下载源选择、前端命令与事件，以及用户可见行为和失败处理。
- 继续统一编译 OEA 二进制。helper 使用该二进制的副本与专用启动参数，不新增独立分发的 helper 可执行文件。

保留统一二进制可以沿用现有发布流程，避免新增 helper 制品及其版本匹配责任。完整抽取使操作状态与安装恢复规则能在一个 crate 内维护。

## 重构规模约束

本次以文件移动、引用路径调整和必要的接口解耦为主。沿用现有内部模块结构、算法和状态机，不借拆 crate 重新设计更新流程。允许保持合理业务语义的小幅局部调整，不要求 CDK 解密、代理解析等依赖准备的失败顺序完全一致。评估 diff 时区分文件移动产生的路径变化与真实逻辑改动。

- 尽量整文件迁移检查、下载、状态管理、候选构造和安装事务实现，保留现有内部命名及可见性层级，仅调整跨 crate 所必需的部分。
- 接口改动限于显式传入路径、版本和更新选项，替换 Tauri 进度及阶段通知，传入 helper 启动配置，以及 OEA 侧传输类型转换。转换保持现有默认值、错误分类和设置快照时机，安装事务关键操作顺序保持不变。
- 保留现有 helper 参数协议、文件替换重试、事务锁、`WaitingForHelper` 行为及失败清理。新增 PID 等待、就绪握手或调整重试策略属于另行讨论的行为变更。
- 前端传输类型转换保持直接、局部，不额外引入通用适配框架。平台代码只迁移更新实际需要的实现，共用能力的处理以实际引用为准。
- 测试尽量随实现迁移，保留现有断言。只补充新接口带来的实质风险验证，不重写整套测试，不添加仅检查文件位置或符号名称的测试。

行为等价需要通过实际 diff 审查、现有测试、前端协议检查和 Windows 更新验收建立证据。设计阶段不预先承诺具体变更行数，也不以通过编译代替行为验证。

## 迁移前的实现

- `crates/oea/src/update/commands.rs` 同时承担 Tauri 命令适配和检查、下载编排。从 `Controller` 取得设置快照，从 `AppHandle` 取得当前版本。
- `check.rs` 使用 Mirror酱查询最新发布版本并判断是否存在可用更新，同时负责从应用设置解密 CDK。`source.rs` 根据设置和缓存元数据解析 Mirror酱、OEM 或 GitHub 下载计划。
- `manager.rs` 管理检查、下载、安装的互斥状态，以及可用更新、待安装更新。租约在失败或取消时恢复状态。
- `download/` 负责临时文件、流式写入、大小与摘要校验、下载包发布。`progress.rs` 直接使用 Tauri Channel。
- `install/` 负责完整候选版本构造、全量与增量包处理、事务发布、helper 交接和启动期恢复。其中混有 Tauri 事件、退出请求、应用路径、语言、日志初始化与原生提示。
- `main.rs` 在正常应用入口前分派 helper 请求。`app/mod.rs` 在创建窗口、OCR 和静态数据消费者前完成资源事务。

安装协议详见[更新安装的磁盘状态](update-installation.md)。旧进程准备完整 candidate 并发布 `transaction.json`，helper 只替换 executable，新版本启动时完成 resources 交接。删除事务标记是提交点。当前协议不提供旧版本回滚。

## 已确认的模块职责

依赖方向为 `oea → oea-update`。crate 名称为 `oea-update`，目录为 `crates/oea-update/`，主要公开对象为 `Updater`。OEA 中现有 `update` 模块保留应用适配职责。

- 更新内核拥有版本判断、下载源解析、下载校验、操作状态机、候选版本构造和安装事务恢复。内部继续使用具体的 HTTP 和文件系统实现。
- OEA 保留 Tauri 命令与传输类型、设置持久化及 CDK 解密、应用变体判断、自动更新触发策略、界面提示和退出编排。
- 通过更新专用输入传入当前版本、更新选项和路径，通过普通 Rust 结果与进度通知返回信息。内核不接收 `AppHandle`、`Controller` 或完整 `OeaSettings`。
- helper 提交和启动恢复需有独立于正常 GUI 初始化的入口。候选文件、事务标记和各阶段的恢复规则由内核统一维护。

### 操作状态与公开接口

由一个有状态的更新对象提供检查、下载、取消下载、安装及状态查询。可用更新、待安装更新、操作互斥、取消与失败清理由内核管理。租约、下载计划和事务阶段保持为内部细节。helper 执行和启动恢复提供独立入口，不依赖正常 GUI 的状态初始化。

`Updater` 的操作包括 `check`、`download`、`cancel_download`、`status`、`is_installing`、`install` 和 `install_local`。本地包安装与在线更新共用操作互斥和安装内核，文件选择及暂存由 OEA 负责。

设置继续在每次操作入口读取当前生效快照，以保持既有行为。检查与下载之间保存了新设置时，下载读取新快照。OEA dev 的更新禁用策略继续生效。

`check` 和 `download` 接收一次性的选项构造函数，在取得内部操作租约后调用，由 OEA 读取设置快照并生成 `Options`。系统代理解析结果保存在选项中，只有实际构建使用代理配置的客户端时才报告解析失败，Mirror酱直连下载不受该错误影响。

### 平台能力

文件锁、原子替换、候选目录、事务恢复及 helper 副本准备归内核。内核私有的平台模块封装所需 Windows 原语，公开接口不泄露 Windows 原生类型。

OEA 负责应用设置读取、CDK 解密、系统代理解析、日志初始化、原生提示和应用退出。内核接收具体路径与更新选项，通过普通 Rust 通知报告进度和阶段。OEA 将通知接到 Tauri 或原生提示窗口。helper 按下文的显式配置启动。

### 前端协议

内核定义普通 Rust 类型，OEA 保留前端传输类型并进行显式转换。`ts-rs` 导出、命令名、事件名及 JSON 编码约定由 OEA 管理。内核内部网络响应和磁盘事务所需的 `serde` 继续保留。

## helper 与宿主的接口

### helper 的宿主无关性

已明确要求：内核不识别调用它的可执行文件，不假定内核或 helper 入口由 OEA 二进制承载，也不规定可执行文件如何封装 helper 入口。入口相关配置由 OEA 显式提供。统一二进制是 OEA 的集成选择。

更新目标与 helper 启动程序分别描述。内核仍需要显式的安装根目录和目标 executable 名称来执行文件替换。helper 启动程序的来源与参数由宿主提供，内核不自行通过 `current_exe()` 推断调用者，也不解释 OEA 私有命令行参数。helper 模式的参数解析及入口分派留在 OEA。现有 helper 路径身份校验仍需保留，具体接口在实施设计中落实。

OEA 提供 helper 源程序路径和原生参数列表，内核负责事务发布、helper 副本准备及按配置启动，并在启动失败时取消事务及清理。启动成功后内核保持安装状态，返回表示 helper 已启动的结果，由 OEA 立即请求应用退出。此处的成功只表示进程已启动，不新增 helper 就绪握手或改变现有安装协议。宿主不应在成功后继续正常业务或撤销已交接的事务。

具体接口采用 `InstallTarget::new(root, executable_name)` 描述安装目标，`HelperLaunch { source_executable, args }` 描述启动配置，`HelperStarted { process_id }` 表示已启动的进程。helper 入口先调用 `validate_helper_request(root, executable_name, actual_path)` 获得已验证请求，宿主再初始化日志并执行请求。启动恢复通过 `complete_startup_transaction` 执行，提交开始的通知用于宿主展示原生提示。

`install` 和 `install_local` 接收一次性的 helper 配置构造函数，在事务准备完成后才读取宿主的语言及源程序路径。配置构造、helper 复制或进程启动失败都由内核按现有规则取消事务。配置构造函数仅返回数据，不负责启动进程或退出宿主。

### 当前进程交接机制核查

当前没有宿主 PID 参数、进程句柄等待、父进程存活检测或 helper 就绪握手。宿主从 `spawn` 取得的 helper PID 只用于日志，不等待子进程退出或读取退出码。helper 参数只携带入口标志、安装根目录、目标 executable 名称和界面语言。

helper 在持有 `transaction.lock` 的情况下尝试替换 executable。替换失败时每隔 100 毫秒重试，重试窗口为 30 秒。Windows 使用 `ReplaceFileW` 替换已有目标，通过文件能否替换间接等待占用释放，不核实某个特定 PID 是否已退出。超出重试窗口后取消事务并返回错误，清理失败会附加到错误信息。

`transaction.lock` 负责跨进程串行化事务文件操作。helper 使用阻塞获取，应用启动恢复使用非阻塞获取。启动恢复若遇到锁冲突会报错并中止正常初始化。若取得锁但 candidate executable 尚未提交，则返回内部状态 `WaitingForHelper`，保留原有 executable/resources 并继续正常初始化。这个状态本身没有等待、轮询或重新启动 helper 的行为，也不会作为更新成功结果发送给前端。

宿主只同步获知 helper 是否启动成功。helper 启动后的执行错误由 helper 写日志、展示提示并以错误退出码结束，不向原宿主回报。因此公开结果应表达 “helper 已启动”，不能表达 “宿主已退出”“helper 已就绪” 或 “更新已完成”。

自动化测试覆盖 helper 前启动的 `WaitingForHelper` 状态，以及依次运行 helper 和新版本启动的子进程事务交接。Windows 发行版真实进程退出时序仍需实机验收。

代码入口：[`helper.rs`](../crates/oea-update/src/install/helper.rs)、[`transaction.rs`](../crates/oea-update/src/install/transaction.rs)、[`startup.rs`](../crates/oea/src/update/install/startup.rs)、[`platform/update.rs`](../crates/oea-update/src/platform/update.rs)。

启动恢复保留一处局部修正：如果资源已移动、只剩删除事务标记，内核不会调用资源移动通知。OEA 在完成后只关闭已经创建的提示，避免原先对不存在的提示执行 `expect` 导致 panic。事务提交条件保持不变。

### 不返回的函数

真实不返回的函数使用 never 类型表达。函数的返回类型必须反映实际控制流。Tauri 的 `AppHandle::exit` 返回 `()`，通过退出事件推进应用生命周期，现有安装调用并不保证永不返回。保留安装操作的成功结果，由 OEA 请求退出，不为使用 `!` 改成强制终止进程。

本机 Rust 1.99.0 的最小编译验证表明，函数及 trait 方法的 `-> !` 可用，`Result<!, E>` 报 E0658，`Result<std::convert::Infallible, E>` 可用。若某个接口确实只在失败时返回，可以使用后一种写法。安装与 helper 执行接口会正常返回结果，因此不采用这一返回类型。

OEA 在 `RunEvent::Exit` 中调用 `BackgroundThreads::shutdown()`，完成事件排空、线程停止及日志 guard 释放。直接 `std::process::exit` 不会运行栈上的析构函数，因此不能把它当作保持当前退出行为的等价替换。

参考：[Tauri AppHandle::exit](https://docs.rs/tauri/latest/tauri/struct.AppHandle.html#method.exit)、[Rust never 类型](https://doc.rust-lang.org/stable/std/primitive.never.html)、[Rust process::exit](https://doc.rust-lang.org/std/process/fn.exit.html)。

## 验收标准

- `oea-update` 可以单独编译和测试，不依赖 `oea` 或 Tauri。
- 迁移现有状态机、取消、下载校验、全量与增量安装、异常中断恢复测试，按新公开接口调整测试入口。
- OEA 侧验证原有命令、事件和 JSON 格式保持兼容。
- Windows workspace 编译、Clippy 和测试通过。
- 完成一次旧发行版到重构后发行版的真实更新验收，覆盖统一二进制的 helper 入口、退出及启动期资源交接。未完成时明确记录为待验收。

macOS 开发检查不替代 Windows 验收。

## 迁移顺序

1. 建立 `oea-update` crate，迁移状态、错误、检查和下载逻辑。OEA 将设置转换为操作输入，并转换前端传输类型。
2. 迁移安装候选构造、事务及所需平台原语，以显式目标路径取代对 OEA `AppPaths` 的依赖。
3. 接入 helper 启动配置与独立执行入口，保留 OEA 的参数解析、提示、日志初始化和退出流程。启动恢复仍先于窗口和资源消费者创建。
4. 完成旧模块清理，迁移行为测试并执行上述验收。迁移期间保持现有发布格式和安装协议。

已存在的领域术语沿用[术语表](GLOSSARY.md)。具体实现选择保留在本文。
