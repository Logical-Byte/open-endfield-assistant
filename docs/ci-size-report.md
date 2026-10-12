# PR 便携 ZIP 体积报告 Spec

状态：实验性 MVP 已实现，待合入默认分支并进行真实 Actions 运行。本文汇总本次讨论的最终决策。Spec 正文使用中文，自动发布的 PR 报告使用英文。

## Problem Statement

Maintainer 在审查 PR 时，需要知道应用的实际下载包会变大还是变小，以及变化主要来自应用程序、OCR 模型还是其他资源。只与 stable release 比较能了解累计发布变化，同时与 base branch 比较能更直接地了解当前 PR 分支与目标分支的体积差异。

完整 Windows release 打包需要 Cargo 编译，耗时较长。每次 push 都增加打包任务成本过高。Maintainer 希望通过 PR 评论按需请求报告，并在同一 PR 多次更新而 base 不变时复用 base 测量结果。

本项目发布物是 Windows x86_64 便携 ZIP，包含应用程序与资源。常规 CI 当前运行前端构建、Cargo 测试和 clippy，没有生成可直接复用的最终 release ZIP。已有 stable Release 则包含实际发布的 ZIP，可直接统计。

## Solution

Maintainer 在打开的 PR 下评论 `/ci-show-artifact-size`，系统固定当前 PR head、base branch tip 和最新 GitHub 正式 Release，按需生成英文体积报告。主要构建流程在同一 Windows job 中串行处理 base 和 PR。

base 对应统计缓存存在时跳过其构建，缺失时构建并保存。PR 每次请求都重新执行完整打包和统计，成功后刷新对应提交的统计缓存。Cargo 缓存始终用于加速。使用 `/ci-show-artifact-size --rebuild-base` 可以忽略 base 统计缓存，强制重新运行 base 打包与测量。

stable 统计先查缓存，未命中才下载实际发布 ZIP。全部统计只报告 ZIP 中的压缩大小。报告用一张表并排展示 stable、base、PR 和两个相对变化列，更新同一条 bot 评论。表格后的数据来源、运行链接和重测提示放入默认折叠的详情块。

后续 push 只将已有报告标记为过期，不自动重新打包。再次发送指令才测量最新提交。

## User Stories

1. 作为 maintainer，我希望通过 PR 评论指令请求体积报告，以便在审查需要时才消耗完整打包资源。
2. 作为 maintainer，我希望普通 PR 创建、描述编辑和 push 不自动触发打包，以便控制 CI 成本。
3. 作为 maintainer，我希望请求只允许有仓库写权限的成员发起，以便避免任意访客消耗 Windows runner。
4. 作为 maintainer，我希望指令只处理打开 PR 下的新评论，以便普通 issue、引用和评论编辑不会意外触发构建。
5. 作为 maintainer，我希望请求开始时固定 PR、base 和 stable 的比较对象，以便报告能追溯到准确版本。
6. 作为 maintainer，我希望 base 与 PR 串行构建，以便 PR 能复用同一 runner 中已编译的依赖。
7. 作为 maintainer，我希望 base SHA 不变时复用其统计，以便连续更新同一 PR 时节省编译时间。
8. 作为 maintainer，我希望 base SHA 改变或缓存缺失时自动补建，以便比较对应当前目标分支。
9. 作为 maintainer，我希望 PR 每次请求都重新打包测量，以便得到这次请求实际生成的产物大小。
10. 作为 maintainer，我希望 PR 每次成功测量都保存最新统计，以便结果能够追溯并进入缓存记录。
11. 作为 maintainer，我希望使用 `--rebuild-base` 重新测量 base，以便主动刷新已有统计。
12. 作为 maintainer，我希望 Cargo 缓存始终帮助构建加速，以便强制测量 base 时仍避免无谓的依赖编译。
13. 作为 maintainer，我希望 stable 使用实际发布 ZIP 的大小，以便与真实发布物比较。
14. 作为 maintainer，我希望 stable 的统计也能缓存，以便多个请求无需反复下载同一发布包。
15. 作为 maintainer，我希望发布新 stable 后下一次请求自动使用新版本，以便基准保持符合当前正式发布状态。
16. 作为 maintainer，我希望已有老版本也能提供包内构成，以便不依赖额外的 release manifest。
17. 作为 maintainer，我希望只查看最终 ZIP 和内部压缩大小，以便数字对应用户下载的包。
18. 作为 maintainer，我希望查看应用程序、OCR 模型、数据与模板及其他文件的变化，以便定位体积增长来源。
19. 作为 maintainer，我希望应用程序明确注明包含前端，以便避免把整个 exe 误认为纯后端。
20. 作为 maintainer，我希望表格同时显示相对 stable 和 base 的差值与变化率，以便分别评估发布累计变化和分支间差异。
21. 作为 maintainer，我希望报告注明基准版本、测量时间和缓存来源，以便判断结果是否适用于当前审查。
22. 作为 maintainer，我希望报告使用英文且将补充信息折叠，以便保持评论清晰紧凑。
23. 作为 maintainer，我希望同一 PR 始终更新一条 bot 报告评论，以便避免反复测量产生大量报告评论。
24. 作为 maintainer，我希望 push 后旧结果被标记为过期，以便不把旧提交的数字当作最新结果。
25. 作为 maintainer，我希望构建期间发生更新也能识别过期结果，以便长时间构建不会产生误导性报告。
26. 作为 maintainer，我希望再次发指令时以最新请求为准，以便旧任务不能覆盖新报告。
27. 作为 maintainer，我希望失败、取消和数据缺失有明确状态及运行链接，以便能够判断原因并重新请求。
28. 作为 maintainer，我希望缓存被清理后按需重新构建，以便无需维护长期统计存储。
29. 作为 maintainer，我希望缺失数据使用 `N/A`，以便未知数据不会被误当成零。
30. 作为 maintainer，我希望 PR 关闭后停止接受测量请求，以便不为已结束的审查产生额外构建。

## Implementation Decisions

### 指令与比较对象

- 支持 `/ci-show-artifact-size` 和带一个 `--rebuild-base` 参数的形式。去掉首尾空白后严格解析，不接受引用中的指令、任意附加参数或其他命令。
- 只处理 PR 下新建评论。PR 描述、checkbox、评论编辑和普通 issue 不触发测量。
- 发起人需要仓库 write、maintain 或 admin 权限。无权限的 PR 作者可以由 maintainer 代为触发。
- 请求开始时通过 GitHub API 固定 source repository、PR head 完整 SHA、base branch 名称与 tip 完整 SHA、stable Release 与对应 ZIP asset。整个请求使用这组快照。
- base 定义为请求时目标分支的 tip，不使用 merge-base 或 PR 合并 ref。PR 构建对象是 head 提交，不是默认 checkout 得到的合并 ref。
- stable 使用 GitHub latest 正式 Release，排除 draft 和 prerelease。使用 API 的 latest 语义，不自行按 semver 或发布时间重新排序。不使用 R2 当前 stable 通道。
- 匹配实际发布的 Windows x86_64 便携 ZIP，排除源码归档和 Actions artifact 外层归档。

### 执行模块与串行构建

- 分为四个逻辑部分：指令接收、串行构建与测量、缓存保存与报告发布、过期标记。预期使用三个 workflow，指令接收和过期处理合并在控制 workflow。
- 控制 workflow 处理评论，校验权限并建立可信请求记录，更新已有或创建新的报告评论，再 dispatch 默认分支的主任务。
- 默认分支的主任务定义负责构建，显式切换到固定 base 和 PR 提交。dispatch 自身的 run SHA 不能当作被测提交 SHA。
- 主要构建在同一个 Windows job 内串行进行。准备 Node、Rust 并恢复 rust-cache，先处理 base，再处理 PR。
- base 统计命中且没有 `--rebuild-base` 时跳过 base 编译和打包。未命中或强制刷新时执行真实 release 打包，测量并输出 JSON。
- base 处理后切换到固定 PR head，更新子模块和依赖，清理前端构建与打包暂存产物，始终重新执行打包、测量。即使 head SHA 与上次相同，也不读取旧统计来跳过此次打包。
- base 与 PR 之间保留 Cargo target，让 Cargo 判断可复用的编译产物。统计 JSON 放在源码切换不会清除的暂存位置。两次打包不混用旧 dist 或 ZIP。
- 复用现有真实 release 打包路径，包括实际发布资源和 OCR 模型，不建立模拟包装流程。不额外固定、校验或维护 OCR 模型版本，接受不同测量时间可能使用不同远端内容。
- 执行 PR 代码的 job 不持有评论写权限、发布 secrets 或仓库写 token，checkout 不持久化凭据。构建结果以数据 artifact 传递。
- 在 dispatch 主任务中设置独立收尾 job，校验请求归属和 JSON 后保存统计缓存。该 job 不执行 PR 代码。stable 缓存的恢复、下载和统计也由可信任务执行。
- 结果发布 workflow 在主任务完成后读取结果，生成报告并更新评论。过期处理只读取 PR 元数据和更新评论，不 checkout 或执行 PR 代码。
- 普通 CI 与现有发版流程保持原来的职责，不增加每次 main push 或 PR push 的打包任务。

### MVP 实现方式

- MVP 的自定义逻辑全部内嵌在 YAML 中，通过 `actions/github-script` 执行 JavaScript，暂不建立独立 CI 工具项目、仓库脚本文件或 Python 脚本。
- YAML 表达触发条件、权限、缓存和串行构建。github-script 处理指令、可信请求、ZIP 统计、JSON 校验、差值计算和英文评论生成。
- ZIP 读取使用 yauzl 3.4.0，安装到 runner 临时目录，关闭安装脚本，不加入应用依赖或修改应用锁文件。读取中央目录与 compressedSize，无需解压。
- 控制和报告逻辑来自默认分支的 workflow，执行评论写入的任务不执行 PR 代码。请求快照编码在 bot 报告的隐藏 metadata 中，后续任务从这条可信评论读取并核对请求 ID。
- 用 YAML anchor 复用临时 ZIP reader 安装和 base/PR 测量代码，不为 MVP 新增共享脚本抽象。
- 如后续需要提取仓库脚本，全部放在根目录 `ci/`，只使用 JS/TS，届时再考虑独立 package 与锁文件。

### 压缩大小与 JSON 数据

- 读取 ZIP 中央目录的每个文件压缩字节数，按类别汇总，无需解压到磁盘。
- 统计包含最终 ZIP 总字节数、应用程序、OCR 模型、数据与模板、其他文件及 ZIP 结构开销。
- 应用程序包含内嵌前端、Rust 代码及依赖。前端与后端不拆开统计。
- OCR 模型单独归类，数据与模板在报告中合并归类，所有剩余文件归入 Other files。
- ZIP 结构开销为 ZIP 总字节数减去文件压缩字节数总和，包括文件头、中央目录等。各构成和结构开销加总应与 ZIP 总大小一致。
- 使用整数字节进行计算，展示时换算 MiB，保留两位小数。报告不显示解压后大小或本次占比。
- 相对 stable 的差值为 PR 减 stable，相对 base 的差值为 PR 减 base。变化率使用对应基准作分母。正数表示变大，负数表示变小。
- 未知值及无法计算的对应差值显示 `N/A`。基准为零时显示绝对差值，变化率显示 `N/A`。明确缺失类别且完整读取的包可以统计为零，未知数据不能当作零。
- 构建统计 JSON 包含格式版本、source repository、source SHA、平台、PR 编号、总字节数、各类压缩字节数、测量时间和 run 身份。stable JSON 包含对应 Release/asset 身份与测量结果。

### base 与 PR 的统计缓存

- 小型统计 JSON 使用 GitHub Actions Cache 保存，不维护持续改写的全量 commit 映射，不要求长期保留 ZIP。
- base 允许恢复匹配记录，PR 不恢复旧统计。每次成功构建都保存新记录，PR 的刷新在逻辑上替代该提交上一次统计。
- 保存 key 为 `oea-size-stats-v1-windows-x86_64-pr-<PR>-commit-<完整SHA>-run-<run_id>-<run_attempt>`。
- base 恢复前缀为 `oea-size-stats-v1-windows-x86_64-pr-<PR>-commit-<完整SHA>-run-`。只在该完整 SHA 的范围内选择最近创建的记录，并核对 JSON 身份。
- `v1` 代表统计格式和分类规则，规则不兼容时升级。PR 编号限定统计复用范围，完整 SHA 区分提交，run ID 与 run attempt 区分成功重测及重试。
- GitHub Cache 已有 key 不能覆盖，因此每次重测使用新 key 保存。只保存成功统计，失败刷新保留先前有效记录，报告明确本次失败。
- 不增加自定义 TTL。GitHub 默认清理超过 7 天未访问的缓存，空间压力也可提前淘汰。缓存缺失时按需补建。
- 保持统计缓存操作的 OS、固定暂存路径和格式一致。主任务统一 dispatch 到默认分支，显式 checkout 不改变 run 的 cache ref 范围。
- cache key 和请求身份由可信控制流程决定，不接受 artifact 自报身份来决定写入位置。保存前检查格式、数值、请求及 run 关联。

### stable 统计缓存

- 每次请求仍查询 Release metadata，确定当前 latest 正式 Release 与 ZIP asset。缓存只省去大文件下载和统计，不省略最新版本发现。
- key 为 `oea-size-stable-v1-windows-x86_64-asset-<asset_id>-<metadata_hash>`，使用精确匹配。
- metadata hash 为 asset 更新时间与大小的 SHA-256，API 提供 digest 时一并纳入。新版本、删除重传或 asset metadata 更新产生新 key。
- stable key 不含 PR 编号，允许多个 PR 共用同一实际发布物的统计。不使用 latest 字符串或仅 tag 作固定 key。
- 命中时读取统计 JSON，未命中时下载实际 stable ZIP、统计并保存。老版本不需要新增 manifest 或重建。
- stable ZIP 下载或解析失败时，asset metadata 可继续提供总 ZIP 大小，内部构成和对应变化显示 `N/A`，详情中说明原因。

### Cargo 缓存

- 始终启用 rust-cache，不支持 `--no-rust-cache`。`--rebuild-base` 只绕过 base 体积统计缓存。
- prefix-key 使用 `oea-size-rust-v1`，shared-key 使用 `pr-<PR>-windows-x86_64-release`，关闭自动 job ID 区分，配置项目现有 Cargo workspace 与 target。
- Rust 版本、Cargo.lock、Cargo.toml 和编译环境等自动 key 组成保留，由 rust-cache 处理。不把 commit SHA 或 run ID 加入 Cargo key，以便同一 PR 更新后仍复用依赖。
- 同一 runner 上保留 target，允许 PR 复用刚完成 base 的兼容产物。rust-cache 默认主要缓存依赖，应用本身仍可能需要编译与链接，不承诺免编译。
- PR 测量使用专用 Cargo 缓存命名空间，可信发布任务不恢复此命名空间。

### 报告、链接与状态

- 自动 PR 评论使用英文，移除 `This message is generated by AI.` 声明。
- 按 bot 自己发布的固定隐藏标记定位报告。同一 PR 始终更新同一条报告评论，不删除重建或追加新报告。
- 表格前展示标题、状态、PR 提交链接、base 分支与提交链接、stable Release 链接、单位与压缩口径说明。
- 表格列为 Component、Stable、Base、PR、Change vs. stable、Change vs. base。两列变化均以 PR 为当前值，单元格同时显示 MiB 差值和百分比。
- 表格后所有来源、测量时间、运行链接和操作提示放入默认折叠的详情块，摘要为 Build details and rerun commands。
- 详情说明 PR 本次构建、base 使用缓存还是本次重建、缓存的原测量时间以及 stable 来自实际发布 ZIP。
- 提供本次 CI run 链接。base 使用缓存时额外提供原测量 run 链接。重测提示包含普通指令及 `--rebuild-base`。
- 状态严格限定为 `working`、`completed`、`outdated`、`failed`，不引入 cancelled 或“working but will be outdated”等复合状态。
- `working` 表示正在处理当前提交的有效请求，显示该请求的 PR/base/stable 身份与运行链接。
- `completed` 表示成功得到当前提交的 PR 测量结果。部分基准缺失时仍可显示 completed，对应列用 N/A 并在详情说明原因。
- `outdated` 表示报告或正在进行的测量不再代表当前提交。保留上次表格时明确它属于上次测量。
- `failed` 表示最新有效请求的 CI 失败，未能产生有效当前结果。正文简短说明失败，折叠详情提供失败阶段或原因以及运行链接。旧统计缓存不被失败结果覆盖，旧数字不能展示为本次成功结果。仅基准数据缺失但 PR 测量成功时使用 completed 与 N/A。
- 旧请求的失败不能覆盖较新的请求。已因 push 标为 outdated 的旧构建，后续失败仍保持 outdated，失败信息放入详情。任务取消由详情说明，不单独增加 cancelled 状态。
- PR push 后立即把已有报告更新为 `outdated`，保留原数字、测量 head SHA、base 和 stable 身份，提示重新发送指令。不要求携带或持续更新最新 head SHA，提供 PR 链接即可。已经 outdated 的报告收到后续 push 时无需重写。没有报告的 PR 不新增提示评论。
- outdated 格式化使用统一的轻量转换：把固定的状态行替换为 `Status: outdated`，在完整正文末尾追加固定的可见提示“This report is outdated. Comment `/ci-show-artifact-size` on an open PR to update it.”。提示位于折叠块外，其余正文保留。completed、failed 和 working 都使用同一转换，不增加分状态的过期模板。
- completed 过期保留原表格和详情，failed 过期保留原失败说明和运行链接。原 PR 提交身份保留，failed 的 SHA 是请求构建对象，不声称已成功测量。固定标记用于识别过期提示，已经添加则不重复追加。再次有效请求时重新生成 working 正文，清除过期提示。
- 构建期间 head 更新也立即标为 outdated，不保持 working 并附加将来过期的预告。旧构建完成后结果仍标为 outdated。相同 head 的多次请求仍用不同请求身份区分，避免旧 run 覆盖最新 run。
- 新指令接替并取消旧请求。缓存与报告写入在可信阶段检查最新请求身份，防止取消竞态或迟到结果回写。
- PR 关闭后不接收新请求，旧任务不能覆盖关闭状态。失败或取消不触发自动无限重试，maintainer 可再次发送指令。

## Testing Decisions

按用户明确要求，本功能不新增或执行测试。不设置测试接口、测试计划或测试门禁。

## Out of Scope

- 每次 push、PR 创建或描述修改自动执行完整包测量。
- PR checkbox、普通 issue 命令、评论编辑触发或任意自然语言解析。
- 独立前端和后端大小、解压后大小、包内占比或二进制符号归因分析。
- 使用 merge-base 或合并 ref 构建作为默认比较对象。
- R2 stable 通道基准、多平台与多架构报告。
- 给每个 Release 新增体积 manifest，或者重新构建历史 stable 获取基准。
- OCR 模型版本治理、跨项目构建数据库、对象存储永久缓存、自定义 TTL 和统计定期刷新任务。
- 禁用 Cargo 缓存的命令、每次 main push 预热 base 或体积预算失败门禁。
- 修改正式发布流程、部署 PR 产物、自动发版或修改应用功能。
- MVP 中建立独立 ci 工具 package 或提取脚本文件。
- MVP 不自动创建 GitHub issue、推送分支或启用尚未合入默认分支的 workflow。

## Further Notes

本 spec 使用 to-spec 的结构整理当前讨论。评论格式与 cache key 已作为实施约定记录。原来的公开评论 AI 声明规则，在本功能的自动体积报告中按本次明确要求省略。

体积比较具有快照性质。PR 与 base 的差值可能包含尚未同步的主线变化，stable 的差值包含该正式版本以来的累计变化。缓存中的数字是此前真实测量值，接受工具链或远端模型随时间变化，以测量时间说明来源。

GitHub Cache 的相关平台约束：已有 key 不可修改，前缀恢复选择最近创建记录，key 最大 512 字符，cache ref 与内部 version 共同影响可见性。默认 7 天闲置清理及容量淘汰使其适合作为加速层。默认分支上的 issue_comment 和 workflow_run 等事件有默认只读缓存限制，因此写缓存在 dispatch 主任务的独立收尾 job 中完成。实施时核对当时官方文档与 action 支持版本。

MVP 已完成本地 workflow 实现及静态语法检查，未执行测试、真实 Windows 打包或发表评论。通过只读 GitHub API 确认当前 latest 正式 Release 为 v1.3.0，其 Windows x86_64 ZIP 名称与匹配规则一致。以下示例数字、提交和链接仍为占位内容。

### English report example

### Windows x86_64 portable ZIP size report

**Status: completed**

- **PR:** [`def5678`](https://github.com/OWNER/REPO/commit/def5678)
- **Base:** [`main@abc1234`](https://github.com/OWNER/REPO/commit/abc1234)
- **Stable:** [`v1.3.0`](https://github.com/OWNER/REPO/releases/tag/v1.3.0)

All sizes are in MiB. Component sizes are compressed sizes within the ZIP.
Changes compare the PR build with each baseline.

| Component                       |    Stable |      Base |        PR |  Change vs. stable |    Change vs. base |
| ------------------------------- | --------: | --------: | --------: | -----------------: | -----------------: |
| **Total ZIP**                   | **82.00** | **82.50** | **83.10** | **+1.10 (+1.34%)** | **+0.60 (+0.73%)** |
| Application (includes frontend) |     30.00 |     30.50 |     31.00 |     +1.00 (+3.33%) |     +0.50 (+1.64%) |
| OCR models                      |     40.00 |     40.00 |     40.00 |       0.00 (0.00%) |       0.00 (0.00%) |
| Data and templates              |      8.00 |      8.00 |      8.10 |     +0.10 (+1.25%) |     +0.10 (+1.25%) |
| Other files                     |      3.90 |      3.90 |      3.90 |       0.00 (0.00%) |       0.00 (0.00%) |
| ZIP overhead                    |      0.10 |      0.10 |      0.10 |       0.00 (0.00%) |       0.00 (0.00%) |

<details>
<summary>Build details and rerun commands</summary>

**Data sources**

- **PR:** Built for this request.
- **Base:** Reused cached measurements from October 12, 2026, 14:30 UTC+8.
- **Stable:** Measured from the published release ZIP.

**Links**

- [Current workflow run](https://github.com/OWNER/REPO/actions/runs/CURRENT_RUN_ID)
- [Original base measurement](https://github.com/OWNER/REPO/actions/runs/BASE_RUN_ID)

**Rerun**

Comment `/ci-show-artifact-size` to measure the latest PR commit.
Add `--rebuild-base` to rebuild and measure the base as well.

</details>

### Primary sources

- [GitHub workflow events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)：评论、dispatch、完成事件与 PR 更新事件。
- [GitHub cache rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)：匹配、不可覆盖、分支范围、权限和淘汰规则。
- [Swatinem rust-cache](https://github.com/Swatinem/rust-cache)：编译缓存配置与默认行为。
- [GitHub Releases API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)：latest 正式 Release 的定义。
- [GitHub Release assets API](https://docs.github.com/en/rest/releases/assets)：实际发布 asset metadata。
- [yauzl](https://github.com/thejoshwolfe/yauzl)：使用 JS 读取 ZIP 中央目录与压缩字节数。
- [github-script](https://github.com/actions/github-script#run-a-separate-file)：调用仓库内 JS 模块并传入 GitHub 客户端。
- [Tauri AssetResolver](https://docs.rs/tauri/latest/tauri/struct.AssetResolver.html)：前端资产内嵌在应用程序中。
- [Prettier size-check workflow](https://github.com/prettier/prettier/blob/main/.github/workflows/check-sizes.yml) 与 [Gemini CLI CI](https://github.com/google-gemini/gemini-cli/blob/main/.github/workflows/ci.yml)：已有项目的 PR 体积报告实践，作为交互参考。

### MVP 使用与交付

- 三个新增 workflow 分别负责控制、串行构建与缓存收尾、英文报告发布。现有常规 CI 和发版 workflow 保持原职责。
- 合入默认分支后，仓库 Actions 允许执行这些 workflow 时，打开 PR 下的评论指令才会生效。在功能分支上编辑文件本身不会启用默认分支的评论入口。
- 首次请求可能构建 base 与 PR 两份。后续 base SHA 不变且统计缓存有效时只构建 PR。
- 状态转换和统计 key 的关联依赖隐藏请求 ID、run 标题及默认分支 workflow 路径。旧请求完成时若报告已属于新请求，发布任务直接跳过。
- 静态检查包括 actionlint、YAML 解析、内嵌 JavaScript 语法和仓库格式规范。按要求不新增或运行测试，真实 Actions 运行及 Windows 打包结果仍待部署后的实验确认。
