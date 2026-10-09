# 数据与共享类型管线

档案数据经过解包、`resources` 生成、后端投影后供业务使用。Rust 到 TypeScript 的类型生成是独立的开发流程。相关类型定义位置见下文。

- 从解包数据到 `resources` 文件：由 `scripts/` 脚本负责。会影响安装包体积。
- 从 `resources` 文件到后端内存：后端单独维护一份 `resources` 的文件 schema，然后投影为后端内存中经过类型检查的数据类型。
- 从后端到前端：主要通过 tauri IPC 传输。`ts-rs` 为这些传输接口提供了统一的 schema 转换，从而避免维护两份类型、枚举等定义。

相关 schema 和类型定义位置：

- 解包数据 schema 定义：[scripts/models/tableCfg/](../scripts/models/tableCfg/)、[scripts/models/json/](../scripts/models/json/)（TS）
- `resources` 文件 schema 定义：生成侧 [scripts/models/resources/](../scripts/models/resources/)（TS），接收侧 [archive/source.rs](../src-tauri/src/data/archive/source.rs)（Rust，只声明读取所需字段）
- 后端数据类型与生成的前端数据类型：[archive/model.rs](../src-tauri/src/data/archive/model.rs)（Rust）、[src/shared/types/generated/archive/](../src/shared/types/generated/archive/)（TS）

前端通过 [src/shared/types/archive.ts](../src/shared/types/archive.ts) 的手写 barrel 导入档案类型，生成文件单独放在 `generated/` 下。

解包数据的输入、资源生成命令及档案标题来源见 [解包数据参考](game-data.md)。

## 档案数据流

图中矩形表示静态文件，六边形表示命令或转换逻辑，圆柱表示运行时内存数据，圆角节点表示业务用途。

```mermaid
flowchart TB
    subgraph source[上游静态文件]
        unpacked["游戏解包表与配置<br/>ENDFIELD_DATA_DIR"]
    end

    make{{"pnpm makedata<br/>或 scripts/makeAllData.ts"}}

    subgraph resources[应用静态 resources 文件]
        prts["resources/data/prts.json"]
        contract["resources/data/archive_contract.json"]
    end

    load{{"后端启动时加载与投影<br/>data::archive::Database"}}

    subgraph backend[后端运行时内存]
        db[("Database<br/>Catalog 与内部索引")]
    end

    ipc{{"Tauri 命令返回 Catalog<br/>前端接收并构建查询索引"}}

    subgraph frontend[前端运行时内存]
        catalog[("Catalog 与前端查询索引")]
    end

    subgraph usage[业务用途]
        ocr("后端 OCR 匹配与纠错")
        view("前端结果展示")
        correction("前端人工纠错与扫描关联")
        export("OEM 收集状态导出")
    end

    unpacked --> make
    make --> prts
    make --> contract
    prts --> load
    contract --> load
    load --> db
    db --> ocr
    db --> ipc
    ipc --> catalog
    catalog --> view
    catalog --> correction
    catalog --> export

    classDef file fill:#f1f5f9,stroke:#64748b,color:#0f172a
    classDef logic fill:#fef3c7,stroke:#b45309,color:#451a03
    classDef backmem fill:#dbeafe,stroke:#2563eb,color:#172554
    classDef frontmem fill:#dcfce7,stroke:#16a34a,color:#14532d
    classDef business fill:#f3e8ff,stroke:#9333ea,color:#3b0764
    class unpacked,prts,contract file
    class make,load,ipc logic
    class db backmem
    class catalog frontmem
    class ocr,view,correction,export business
```

## 类型生成流

部分前端、后端共用的类型通过 `ts-rs` 统一从 rust 类型编译为 ts 类型。这些生成的文件进入 Git 版本管理。

```mermaid
flowchart LR
    rust["类型、枚举定义<br/>Rust 侧"]
    generate{{"cargo test<br/>或 pnpm generate:types<br/>通过 ts-rs 导出测试"}}
    ts["静态生成文件，需要提交<br/>src/shared/types/generated/archive/"]
    use("前端编译时使用")

    rust --> generate --> ts --> use
```

运行 `cargo test` 命令时，ts-rs 导出测试将会运行，以副作用直接更新生成文件。[package.json](../package.json) 中的 `pnpm generate:types` 直接调用 `cargo test --lib export_bindings`，可以单独运行这些导出测试。默认输出的根目录位置目前由 [.cargo/config.toml](../.cargo/config.toml) 指定。

## 增加其他数据领域

新增数据领域时，可以沿用 `resource` 生成、后端从 `resource` 投影、定义并生成前端契约的流程，分别拥有自己的 namespace 和生成文件目录。
