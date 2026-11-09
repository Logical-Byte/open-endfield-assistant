# 更新测试宿主

`oea-update-test-host` 是一个仅用于测试的极简宿主，通过 `oea-update` 的公开接口验证更新模块的端到端行为。它提供可独立运行的 V1/V2 程序和必要的宿主适配，不依赖 OEA 界面。

在仓库根目录运行：

```bash
cargo test -p oea-update-test-host --test installation -- --nocapture
```

维护时保持宿主轻量，更新逻辑留在 `oea-update` 中。具体测试场景以 [installation.rs](tests/installation.rs) 为准。
