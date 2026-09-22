# 前端 Settings 事务模块重构

## Problem Statement

OEA 前端当前直接导出全局可写的 `ref<OeaConfig>`。设置页、更新弹窗、扫描提示和更新编排因此能够读取或修改完整持久化 DTO，并共同依赖其内部编码、deep watch 自动保存、CDK 明密文转换和失败回滚行为。

用户操作设置控件后，Vue 会立即修改共享响应式对象。deep watch 随后启动异步 `save_oea_config` 调用，但多个 watch 回调之间没有串行关系。快速拖动滑块、连续输入文本或短时间内修改多个开关时，可能出现多个前端保存同时在途、提交快照与成功快照不对应、`saving` 提前结束以及失败回滚覆盖更新编辑等问题。

更新编排还会直接读取前端正在编辑的 `oeaConfig`，使尚未保存成功的自动下载或自动安装选项提前影响可用更新、下载操作和待安装更新的处理。这与仓库已经记录的配置语义不一致：Rust `Controller` 中最近成功保存的配置是已生效配置的事实来源。

Mirror 酱 CDK 的明文变化、异步加密和完整配置保存目前分成两个 watch 阶段。连续输入可能让旧加密请求晚于新请求完成，并把旧密文写回配置。扫描提示的布尔语义也以 `scanTipsDismissedVersion` 和当前提示版本号的形式泄漏到多个 UI 调用者。

设置相关代码同时散落在 `src/utils/app/config.ts`、`src/types/oeaConfig.ts`、`src/utils/uiScale.ts`、设置页面组件、更新弹窗和扫描提示中。展示选项、持久化 DTO、设置领域投影、保存编排、错误呈现和 UI 控件适配缺少清晰的归属。

## Solution

建立 `src/modules/settings/`，集中组织 OEA 的设置领域代码。模块内部维护一份全局 settings 单例，以统一、扁平的逻辑 `settingsDraft` 作为用户最新编辑意图，以只读 `effectiveSettings` 表示最近一次成功保存并已生效的设置。

所有 OEA 配置修改经过同步的 draft setter。setter 负责归一化与校验、增加 draft revision，并在 writer 空闲时启动 single writer。writer 每次捕获不可变 candidate，按顺序完成 CDK 加密、持久化 DTO 构造和 `save_oea_config`。任意时刻最多存在一个配置保存调用。

writer 忙碌期间，UI 可以继续编辑同一份全局 draft。中间 revision 可以合并；当前 candidate 完成后，writer 再捕获最新 draft。保存成功后才把同一个 candidate 发布为 `effectiveSettings`。更新编排只读取 `effectiveSettings`，因此尚未保存成功的编辑不会驱动自动下载或自动安装。

保存失败时保留用户 draft，并保持 effective 为最近成功值。相同失败 revision 不自动无限重试；用户可以重试、恢复最近成功值，或者通过新的编辑产生新 revision。旧 candidate 失败时如果已经存在更新 revision，writer 继续尝试最新 draft；最新 revision 失败后停止。

控件的输入中间状态、focus、blur、debounce 和 throttle 属于控件局部状态。Switch 和 Select 立即提交 edit；代理 URL 与 CDK 在 blur 或 Enter 时提交；音量滑块实时显示并将合法数字写入 draft，由 single writer 合并中间值。

更新弹窗删除重复的下载设置表单。齿轮按钮关闭弹窗并跳转到设置页的“更新设置”区域。设置页成为 OEA 配置的唯一编辑入口。

UI 缩放仍使用 WebView 自己的状态和持久化机制，不加入 OEA 配置事务。其代码与数值归一化逻辑移动到 settings 目录，以统一用户设置代码的文件归属。

## User Stories

1. 作为 OEA 用户，我希望设置修改立即反映在刚刚操作的控件中，从而获得流畅的交互反馈。
2. 作为 OEA 用户，我希望快速连续修改最终落在我的最后选择上，从而避免滑块或输入框的中间值覆盖最终结果。
3. 作为 OEA 用户，我希望保存进行中仍可继续修改其他设置，从而不让磁盘延迟阻塞设置页。
4. 作为 OEA 用户，我希望保存失败时应用保留我的最新草稿，从而不因临时持久化错误丢失输入。
5. 作为 OEA 用户，我希望能重试失败的设置保存，从而无需重新填写所有设置即可恢复。
6. 作为 OEA 用户，我希望保存失败后能恢复最近一次已保存的设置，从而可以主动放弃无效或不想保留的草稿。
7. 作为 OEA 用户，我希望设置页显示正在保存、已保存或保存失败，从而理解后台持久化状态。
8. 作为 OEA 用户，我希望在设置页外触发保存失败时也收到可见通知，从而避免错误静默发生。
9. 作为 OEA 用户，我希望自动更新行为只在对应设置保存成功后改变，从而避免未提交编辑启动下载操作或安装。
10. 作为 OEA 用户，我希望更新弹窗聚焦于可用更新和下载操作，从而保持其主要用途清晰。
11. 作为 OEA 用户，我希望更新弹窗的设置按钮带我前往唯一的更新设置表单，从而避免遇到两份逐渐漂移的表单。
12. 作为 OEA 用户，我希望所选更新源与其他配置经过同一有序保存流程，从而可靠保存最终选择。
13. 作为 OEA 用户，我希望完成自定义代理 URL 编辑后才提交，从而避免把未输入完成的文本视为完整设置。
14. 作为 OEA 用户，我希望在代理 URL 输入框按 Enter 时提交，从而获得明确的键盘完成操作。
15. 作为 OEA 用户，我希望完成 Mirror 酱 CDK 编辑后再加密和保存，从而避免每次按键都启动一次加密请求。
16. 作为 OEA 用户，我希望旧 CDK 加密工作在新加密工作开始前完成，从而避免陈旧密文覆盖最新 CDK。
17. 作为 OEA 用户，我希望清空 CDK 时可靠清除持久化密文，从而避免意外保留旧秘密。
18. 作为 OEA 用户，我希望 CDK 加密或保存失败时维持最近有效的秘密，从而避免失败编辑破坏更新下载。
19. 作为 OEA 用户，我希望声音音量只接受 `0..1` 内的有限数值，从而避免 UI 库中间值破坏配置保存。
20. 作为 OEA 用户，我希望 UI 缩放始终处于支持范围内，从而避免异常滑块值让界面不可用。
21. 作为 OEA 用户，我希望 UI 缩放保留现有 WebView 持久化行为，从而不因 settings 重构改变缩放的存储位置和时机。
22. 作为 OEA 用户，我希望扫描提示设置表现为简单的启用或停用偏好，从而无需理解持久化版本号。
23. 作为 OEA 用户，我希望确认当前扫描提示后在当前提示版本内不再显示，从而只在提示版本变化后再次看到它。
24. 作为 OEA 用户，我希望在设置页重新启用扫描提示后再次看到当前指引，从而可以重新阅读操作说明。
25. 作为 OEA 用户，我希望仅在本次会话关闭扫描提示仍然是临时 UI 操作，从而不会静默改变持久化设置。
26. 作为前端维护者，我希望只有一个统一逻辑草稿接口，从而让设置控件不再依赖持久化 DTO。
27. 作为前端维护者，我希望只有一个只读的已生效设置接口，从而让业务模块区分已保存设置和待提交编辑。
28. 作为前端维护者，我希望完整配置版本字段留在持久化实现内，从而让 UI 修改无需理解存储兼容性。
29. 作为前端维护者，我希望 `mirrorchyanCdkEncrypted` 留在持久化实现内，从而让普通调用者无法直接写入密文。
30. 作为前端维护者，我希望每次编辑都经过 settings setter，从而无法绕过校验、revision 跟踪和 writer 启动。
31. 作为前端维护者，我希望最多只有一个配置保存正在进行，从而避免完成顺序产生陈旧的 effective 状态。
32. 作为前端维护者，我希望每次保存使用不可变 candidate，从而让后续 UI 编辑无法改变在途事务的含义。
33. 作为前端维护者，我希望合并中间 revision，从而高效持久化设置状态且不把它误建模为操作日志。
34. 作为前端维护者，我希望 writer 完成时重新检查待保存编辑，从而不会漏掉恰好发生在 worker 收尾附近的修改。
35. 作为前端维护者，我希望失败 revision 停止自动重试，从而避免持续存储错误形成重试循环。
36. 作为前端维护者，我希望新的编辑或显式重试可以重新启动 writer，从而保留恢复能力。
37. 作为前端维护者，我希望更新编排只读取已生效的自动下载和自动安装设置，从而让决策与 Rust `Controller` 快照一致。
38. 作为前端维护者，我希望 settings 持久化继续使用现有完整 DTO command，从而避免本次重构引入第二套后端更新协议。
39. 作为前端维护者，我希望现有 Rust `ConfigStore` 原子替换和发布顺序继续作为权威，从而让后端读取者持续获得已提交快照。
40. 作为前端维护者，我希望 settings 专用组件与逻辑集中在 `src/modules/settings/`，从而更容易发现和局部修改相关代码。
41. 作为前端维护者，我希望 `src/pages/settings.vue` 保持为薄路由入口，从而让路由职责与 settings 实现分离。
42. 作为前端维护者，我希望调用者通过 `@/modules/settings` 导入设置，从而由 `index.ts` 记录预期模块接口。
43. 作为前端维护者，我希望第一个模块实现依靠导入约定而不新增全局 ESLint 架构规则，从而保持 PR 聚焦。
44. 作为测试作者，我希望使用内存 persistence adapter 创建隔离的 settings 实例，从而让测试不共享 revision 或在途工作。
45. 作为测试作者，我希望控制持久化 Promise 的完成时机，从而确定性验证重叠编辑和 writer 收尾。
46. 作为审查者，我希望 settings 模块接口和可观察状态变化成为主要测试表面，从而让测试经得起内部重构。
47. 作为审查者，我希望配置文件格式和 Rust command 合同保持不变，从而不把前端正确性修改与迁移风险混在一起。
48. 作为审查者，我希望重复的更新设置表单在同一 PR 中删除，从而让调用者立即使用新的唯一 settings 接口。

## Implementation Decisions

- Add a top-level frontend module at `src/modules/settings/`. Use the plural `settings` because it owns a collection of application preferences and matches the repository's existing user-facing terminology.

- Use the following initial file layout. Keep `settings.ts` cohesive until its implementation size demonstrates a need to extract a separate writer file.

  ```text
  src/
  ├── modules/
  │   └── settings/
  │       ├── index.ts
  │       ├── model.ts
  │       ├── settings.ts
  │       ├── persistence.ts
  │       ├── uiScale.ts
  │       ├── settings.test.ts
  │       └── components/
  │           ├── SettingsCard.vue
  │           ├── SettingsItem.vue
  │           └── DeveloperSettings.vue
  └── pages/
      └── settings.vue
  ```

- Keep `src/pages/settings.vue` as the route entry and page composition layer. Move settings-specific components into `src/modules/settings/components/`.

- Use `src/modules/settings/index.ts` as the intended external interface. Callers import from `@/modules/settings`. Do not add a new `no-restricted-imports` rule in this PR.

- Use an internal singleton in production. Do not introduce Pinia or expose a `useSettings()` composable. Vue ES module evaluation already provides one instance per WebView realm, and the settings state does not follow component mount/unmount lifetimes.

- Expose this public interface. This contract records the agreed interface shape; implementation-only factory and persistence types are not re-exported from `index.ts`.

  ```ts
  export const settingsDraft: SettingsDraft;
  export const effectiveSettings: DeepReadonly<SettingsSnapshot>;
  export const settingsStatus: DeepReadonly<SettingsStatus>;

  export function initializeSettings(): Promise<void>;
  export function retrySettingsSave(): void;
  export function discardSettingsDraft(): void;
  ```

- Use one flat logical draft rather than per-caller projections or nested setting groups.

  ```ts
  export interface SettingsDraft {
    minimizeToTray: boolean;
    soundVolume: number;
    updateSource: UpdateSource;
    mirrorchyanCdk: string;
    updateProxyMode: UpdateProxyMode;
    updateProxyUrl: string;
    autoDownloadUpdates: boolean;
    autoInstallUpdates: boolean;
    scanGuideEnabled: boolean;
  }
  ```

- `SettingsSnapshot` is the read-only logical effective shape. It does not contain `majorVersion`, `minorVersion`, `mirrorchyanCdkEncrypted` or `scanTipsDismissedVersion`. It may retain the logical plaintext CDK because the unified logical settings interface was chosen; the ciphertext representation remains private.

- Do not expose the private mutable raw draft. Construct `settingsDraft` from writable computed fields or an equivalent controlled proxy. A write such as `settingsDraft.soundVolume = 0.8` must synchronously pass through the module's edit path, including validation, revision increment and writer scheduling.

- The edit path contains no `await`. JavaScript run-to-completion semantics make raw draft modification, revision increment and writer scheduling atomic relative to other callbacks in the single WebView realm. Do not add a JavaScript mutex.

- Define single-writer semantics as follows:

  ```text
  edit(patch)
    normalize and validate patch
    synchronously update rawDraft
    increment draftRevision
    clear the previous failed-revision marker
    ensure a writer is scheduled

  writer
    while an unsaved revision exists
      capture targetRevision
      capture an immutable candidate from rawDraft
      encode/encrypt candidate
      await save(candidate DTO)
      on success, publish the same candidate as effective
      if a newer draft revision exists, continue

  writer finalization
    clear the writer latch
    recheck for pending work to avoid a missed wake-up
  ```

- At most one `save_oea_config` call may be in flight from the frontend module. Intermediate revisions are latest-value state and may be coalesced. Settings persistence is not an event log.

- Represent a save with an immutable logical candidate and a corresponding immutable persisted DTO. Never pass a live Vue proxy or read current raw draft after an `await` to decide which snapshot succeeded.

- On successful save, update both the internal last-persisted DTO and public `effectiveSettings` from the saved candidate. The Rust `ConfigStore` remains the authoritative effective configuration; the frontend effective projection mirrors its most recently acknowledged snapshot.

- On failed save, leave `effectiveSettings` unchanged and keep the latest draft. Mark the failed revision and expose a structured error in `settingsStatus`. Do not automatically retry the same revision indefinitely.

- If a candidate fails after a newer draft revision already exists, continue with the newest candidate. If the newest candidate also fails, stop. A later edit or `retrySettingsSave()` clears the failed marker and schedules the writer.

- `discardSettingsDraft()` resets the entire logical draft to the latest effective snapshot and clears the save error. It is a global discard because the module exposes one unified draft.

- Model settings status as structured state sufficient for presentation without importing Nuxt UI into the module. The exact discriminated-union spelling may vary, but it must distinguish loading, idle/saved, saving, load failure and save failure where those states remain observable.

- The settings page displays one global save indicator instead of applying a global `saving` spinner to every individual control. Controls remain interactive while the writer is busy. A save error displays retry and restore actions. The application presentation layer may emit one toast for a newly observed failure, including failures triggered outside the settings page.

- The settings module must not call `useToast()` or render presentation directly. It publishes status and operations; the UI owns wording, placement and toast behavior.

- Retain the existing frontend default DTO and current initialization fallback behavior in this PR. Move the default and persisted DTO into `persistence.ts`. Eliminating duplicated Rust and TypeScript defaults remains separate work under #137.

- Keep the existing Tauri `load_oea_config` and `save_oea_config` contracts. Continue saving a complete DTO. Do not add field-specific commands or a generic patch command.

- Construct each persisted candidate by combining the logical candidate with internal persistence data. Preserve configuration version fields according to the current format, and map the logical scan guide boolean and plaintext CDK to their persisted encodings.

- Move the TypeScript persisted `OeaConfig` type and its defaults behind the persistence implementation. UI and update orchestration must not import this type.

- Put CDK trim, encryption, ciphertext reuse and clearing into candidate encoding. If the candidate plaintext equals the effective plaintext, reuse the most recently persisted ciphertext. If it differs, encrypt it inside the single writer before saving. An empty trimmed CDK produces an empty ciphertext.

- CDK encryption and configuration saving belong to the same ordered commit. Do not retain a separate watch that asynchronously writes `mirrorchyanCdkEncrypted` into a shared DTO.

- Keep Switch and Select edits eager. They write to `settingsDraft` on each completed control change.

- Keep `soundVolume` visually responsive during dragging. Reject arrays and non-finite values before they enter the global draft; clamp finite numbers into `0..1`. The single writer coalesces rapid intermediate revisions.

- Give `updateProxyUrl` and `mirrorchyanCdk` inputs local buffers. Commit their values to `settingsDraft` on blur or Enter. Do not add a time-based debounce in the initial implementation.

- Do not introduce new proxy URL validation rules in this refactor. Preserve the current product semantics for strings and enum combinations.

- Represent the persisted scan guide version as `settingsDraft.scanGuideEnabled`. Map `true` to the existing re-enabled representation and `false` to the current guide version. Callers do not read or write `scanTipsDismissedVersion`.

- Keep `dismissedThisSession` local to `ScanGuide.vue`. It represents temporary component/session UI state and does not enter the global settings module.

- Move `CURRENT_SCAN_TIPS_VERSION` and its mapping knowledge to the settings implementation area while keeping the guide content and UI in the scan feature. Updating guide content and version must remain discoverable through comments or colocated references.

- Remove the nested update settings form from `UpdatePopover.vue`. Keep the settings button, but make it close the popover and navigate to the update section of `/settings`. The update popover continues to own available-update presentation and download-operation controls.

- Move `updateSourceItems`, `proxyModeItems` and shared update-setting labels to the canonical settings presentation area as appropriate. They are presentation data, not persistence DTO fields.

- Change update orchestration to read `effectiveSettings.autoDownloadUpdates` and `effectiveSettings.autoInstallUpdates`. It must not read `settingsDraft` when deciding whether to start an automatic download operation or act on a pending update.

- Move `src/utils/uiScale.ts` into `src/modules/settings/uiScale.ts` and keep its existing WebView persistence path. It is organized under settings but remains a separate state and transaction resource. Do not add it to `SettingsDraft` or promise atomicity with `save_oea_config`.

- Apply finite-number and range normalization to UI scale near its own setting interface. Do not introduce a generic form framework solely to share a few validation lines with sound volume.

- On real process exit, use best-effort persistence. Do not add an exit confirmation, writer drain protocol or dirty-state shutdown blocker in this PR.

- Do not add frontend revision/CAS to the Rust command. The application currently uses one WebView settings writer, while the Rust `CachedJsonFile` already serializes file replacement. Multi-window, multi-WebView and external writer coordination remain future work.

- Internally define `createSettingsModule(persistence)` or an equivalent factory. Production instantiates it once with the Tauri adapter. Tests instantiate fresh modules with an in-memory adapter and controlled promises. The factory is an internal seam and is not re-exported from `index.ts`.

## Testing Decisions

- Test through the highest settings module interface available from a created settings instance: draft edits, effective projection, status, retry/discard operations, and persistence adapter observations. This is the primary test seam.

- Use an in-memory persistence adapter as the second adapter at the internal persistence seam. It must support controlled load results, controlled encryption results, controlled save completion/failure, saved-candidate inspection and concurrent-call counting.

- Use deferred promises to hold a save or encryption step in flight while later edits occur. This makes single-writer behavior deterministic without timers or real Tauri IPC.

- Verify that no more than one configuration save is in flight at any time.

- Verify that a candidate is an immutable snapshot and does not change when the draft is edited during persistence.

- Verify latest-value coalescing: rapid edits while a candidate is in flight eventually persist the final draft without requiring every intermediate state to be saved.

- Verify writer shutdown rechecks pending work, so an edit near finalization cannot remain unsaved while the writer is idle.

- Verify that successful persistence publishes the exact saved candidate as effective and advances the internal persistence baseline.

- Verify that update orchestration observes the old effective automatic-download and automatic-install values until save success, then observes the new values.

- Verify that save failure keeps the latest draft, preserves the previous effective snapshot and publishes an error status.

- Verify that the same failed revision does not automatically retry forever.

- Verify that `retrySettingsSave()` retries the current draft and that `discardSettingsDraft()` restores the complete latest effective snapshot.

- Verify that when candidate B fails after draft D already exists, the writer attempts D; if D fails, it stops with D preserved as the draft.

- Verify CDK serialization: encryption and save do not overlap across candidates, stale encryption cannot overwrite a later edit, unchanged plaintext reuses ciphertext, changed plaintext is encrypted, and clearing plaintext clears ciphertext.

- Never put real CDKs or representative secrets into test fixtures or failure output.

- Verify scan guide mapping through logical behavior: unseen/current-version state, acknowledging the current version, re-enabling the guide, and a newer current version becoming visible. Do not assert private version variables directly when the same behavior can be observed through the logical interface.

- Verify sound-volume normalization: arrays, `NaN` and infinite values do not enter the global draft or persistence; finite values are clamped to `0..1`.

- Verify UI-scale normalization through its own public setting interface and adapter. Do not pretend UI-scale writes are part of the OEA configuration transaction.

- Verify load success, current frontend fallback behavior and public status transitions without changing the duplicated-default policy in this PR.

- Do not write change-detection tests that merely assert `OeaConfig` is not exported or that a particular private file exists. Type checking and migrated callers establish the compile-time surface; behavior tests establish the module contract.

- Do not test private revision counters, writer Promise identity, deep-watch removal or internal clone helpers. Tests should remain valid if the implementation changes while preserving the agreed interface and behavior.

- Prior art is limited on the frontend: the repository has Vitest installed and uses it for `scripts/publishR2Core.test.ts`, but there is no existing general frontend settings test suite. Follow its Vitest conventions where applicable. Rust `CachedJsonFile` tests provide prior art for observable save/load behavior and committed snapshot publication, while this spec adds frontend orchestration tests at the settings interface.

- Run focused tests with a command such as `pnpm exec vitest run src/modules/settings/settings.test.ts` until a project-level frontend test script is established.

- Run the repository frontend verification commands after implementation:

  ```bash
  pnpm fix
  pnpm build
  pnpm check
  ```

- Because persisted DTO construction and Tauri command bindings remain type-coupled to Rust configuration, run the repository-appropriate Windows backend checks if implementation changes any Rust or Tauri command code. Pure frontend relocation with unchanged Rust command contracts does not require new Windows runtime claims.

## Out of Scope

- Eliminating the duplicate configuration defaults maintained by Rust and TypeScript. This remains #137.
- Changing the JSON configuration format, field names, major version or minor version solely for this refactor.
- Adding configuration migrations or code generation between Rust and TypeScript.
- Replacing `save_oea_config` with field-level commands, a patch protocol or a new backend transaction interface.
- Introducing Pinia, another state-management dependency, a generic repository hierarchy, a command bus or a general-purpose task queue.
- Building a generic form framework for debounce, blur, validation or numeric slider adaptation.
- Adding new product validation for proxy URLs, CDKs or update-source combinations.
- Changing Mirror 酱 encryption algorithms, DPAPI scope, subscription rules or user-facing service policy.
- Changing when Rust backend commands clone and consume the effective `Controller` configuration.
- Making UI scaling part of the OEA configuration DTO or promising atomic persistence across WebView zoom and `oea_config.json`.
- Adding exit blocking, an unsaved-changes dialog or a guaranteed writer drain during application shutdown.
- Supporting concurrent settings writers across multiple WebViews, multiple windows, multiple application processes or external JSON editors.
- Adding backend revision numbers, compare-and-swap behavior or conflict-resolution UI.
- Redesigning the overall update popover, available-update state machine, download operation or pending-update behavior beyond removing the duplicated settings form and linking to canonical settings.
- Changing scan guide content or visual design.
- Solving every possible form-field conflict between simultaneous focused local buffers. The canonical settings page becomes the only configuration editor after the update popover form is removed.
- Adding a global ESLint rule for all future `src/modules/*` imports.
- Publishing, updating, labelling or closing GitHub Issues as part of writing this local spec.

## Further Notes

- This spec consolidates the accepted decisions associated with Issues #130, #131, #132, #133, #134, #135, #136 and #138. A complete implementation is expected to satisfy and close those issues. Issue #137 remains open.

- The main depth of `modules/settings` comes from hiding persistence DTO knowledge, Vue write interception, validation, revision tracking, latest-value coalescing, CDK encoding, single-writer orchestration, effective publication and failure recovery behind one logical draft interface.

- The public interface is intentionally broader on read access than the earlier per-caller projection proposal. The chosen simplicity trade-off exposes one unified logical draft and one unified read-only effective snapshot. It still removes the dangerous full writable persistence DTO.

- `settingsDraft` is a logical user-settings interface. It is not the serialized configuration object, even if several field names remain similar.

- Frontend single-writer serialization and Rust `CachedJsonFile` serialization solve different problems. The frontend orders logical candidates and effective publication; Rust serializes file replacement and publishes the committed `Controller` snapshot.

- The design relies on one JavaScript realm for draft edits. If OEA later adds additional WebViews or settings writers, the persistence interface must be revisited with backend revision/CAS semantics.

- The local spec was produced without publishing any GitHub content. It should remain the implementation reference until the resulting PR documents or supersedes the decisions.
