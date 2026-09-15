//! 用户侧本地数据的通用持久化机制。
//!
//! 本模块处理 JSON 序列化、缓存、并发访问和文件提交，不解释模型的领域语义。
//! 具体数据的校验、默认值与迁移策略由使用它的上层模块负责。

mod cached_json_file;

pub(crate) use cached_json_file::CachedJsonFile;
