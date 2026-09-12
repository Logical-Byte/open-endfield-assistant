# 更新安装的磁盘状态

本文从完整 candidate 构造完成开始，说明 executable 和 `resources/` 如何进入应用根目录。
更新只修改这两个目标；`config/`、`logs/` 和其他缓存不参与版本切换。

## 0. Candidate 已完整构造

v1 仍在运行，根目录仍是完整 v1。新版本先在临时目录中完成并校验，再原子 rename 为
`candidate/`。此时还没有正式事务，任何失败都可以直接清理 candidate。

```text
<root>/
├── OEA.exe / OEA                 v1
├── resources/                    v1
└── cache/update/candidate/
    ├── OEA.exe / OEA             v2
    └── resources/                v2
```

## 1. Transaction 已发布

v1 最后以原子 rename 写入 `transaction.json`。它的存在证明 candidate 发布时是完整的，
也表示 executable 与 resources 从现在起可能处于不同版本。

```text
cache/update/
├── transaction.json              事务开始标记
├── transaction.lock              可能存在；helper 与 OEA 启动阶段使用
├── candidate/
│   ├── OEA.exe / OEA             v2
│   └── resources/                v2
└── baseline/                     仅增量更新可能存在
```

## 2. Helper 已提交 executable

v1 启动缓存中的 helper 副本并退出。helper 只做一次原子替换：

```text
candidate/executable  ──rename/replace──>  root/executable
```

```text
<root>/
├── OEA.exe / OEA                 v2
├── resources/                    仍是 v1
└── cache/update/
    ├── transaction.json          仍存在，事务尚未完成
    ├── helper / helper.exe
    └── candidate/
        └── resources/            v2
```

candidate executable 的消失就是交接记录。helper 不移动 resources，因为失败会引入更多
中间状态，而 v2 在使用资源前无论如何都必须检查并完成资源交接。

## 3. v2 已让旧 resources 退出根目录

用户启动 v2。它在创建任何资源消费者前执行第一次原子 rename：

```text
root/resources  ──rename──>  cache/update/discard/resources

<root>/
├── OEA.exe / OEA                 v2
└── cache/update/
    ├── transaction.json          保证意外退出后仍会继续
    ├── candidate/resources/      v2
    └── discard/resources/        v1
```

根目录暂时没有 resources，但正常 GUI 和资源消费者尚未启动。若此时退出，下次 v2 启动
会根据 candidate 与 discard 的位置继续。

## 4. v2 已提交新 resources

v2 执行第二次原子 rename：

```text
cache/update/candidate/resources  ──rename──>  root/resources

<root>/
├── OEA.exe / OEA                 v2
├── resources/                    v2
└── cache/update/
    ├── transaction.json          即将删除
    └── discard/resources/        v1，待清理
```

## 5. 恢复正常启动

candidate 为空后，v2 删除 `transaction.json`。这个删除动作是整个事务的提交点。随后
baseline、discard、candidate 和 helper 都只做尽力清理，清理失败不影响 v2 启动。

```text
transaction 不存在
root executable == v2
root resources  == v2
正常初始化 WebView、OCR、资源消费者和 GUI
```
