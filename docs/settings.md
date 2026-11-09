# 设置存储与迁移

## 职责边界

`oea-settings` 维护设置格式、历史默认值、迁移和持久化。文件路径、系统语言、凭据加解密和应用操作由 `oea` 集成。这使历史文件的解释不依赖 Tauri 启动或系统凭据环境，可以直接用文件样例验证。

设置文件可能由较新版本写入，也可能暂时不可读。初始化失败时不能用可写的默认设置替代它，否则后续普通保存可能覆盖用户数据。应用集成应保留错误和重试通道，存储可用后再执行依赖设置的操作。入口见 [`SettingsAccess`](../crates/oea/src/settings.rs)。

迁移后的设置必须成功写盘才能发布为内存快照，避免内存与磁盘采用不同格式。失败时保留原文件，修复后重试。存储入口见 [`SettingsStore::open`](../crates/oea-settings/src/store.rs)。

## 修改设置格式

设置格式版本由 `majorVersion` 和 `minorVersion` 表示，与应用发行版本独立。用户修改字段值不改变格式版本。

- 不兼容的格式变更增加 major。
- 可迁移的格式或解释规则变更增加 minor，包括新增持久化字段。
- 已发布的迁移规则应保持稳定，使同一历史文件的升级结果可复现。新的语义调整应通过新的迁移表达。

扩展设置时：

1. 在 [`model.rs`](../crates/oea-settings/src/model.rs) 修改当前类型、默认值及格式版本常量。
2. 在 [`migrations.rs`](../crates/oea-settings/src/migrations.rs) 添加来源版本到目标版本的转换，并登记到 `MIGRATIONS`。转换只处理 JSON，不执行文件、网络或凭据操作，便于独立测试并在失败时保留原始数据。
3. 将该历史版本允许省略的字段及其默认值固定在对应补全函数中。历史迁移不能引用当前 `OeaSettings::default()`，否则未来调整默认值会改变旧文件的含义。后续重命名字段时先补全来源版本，再转换字段名，复用该版本的默认值定义。
4. 通过 [`SettingsStore::open` 的集成测试](../crates/oea-settings/tests/settings.rs) 覆盖历史文件、允许省略的字段、多步升级以及失败后原文件的保留。新增版本时保留已有样例，避免只验证最近一个版本的升级。
5. 按 [共享类型管线](data-pipeline.md) 重新生成前端类型。

运行设置存储测试：

```bash
cargo test -p oea-settings
```
