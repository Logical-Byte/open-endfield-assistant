//! 上游资源的私有读取结构，只保留投影所需字段。

use super::{AcquisitionMethod, ArchiveId, Category, Page};
use indexmap::IndexMap;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};
use std::{fmt, hash::Hash, marker::PhantomData};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub(super) struct FirstLvId(pub String);

#[derive(Deserialize)]
pub(super) struct Prts {
    #[serde(rename = "PrtsPage", deserialize_with = "unique_map")]
    pub pages: IndexMap<Page, SourcePage>,
    #[serde(rename = "PrtsCategory", deserialize_with = "unique_map")]
    pub categories: IndexMap<Category, SourceCategory>,
    #[serde(rename = "firstLv", deserialize_with = "unique_map")]
    pub first_lv: IndexMap<FirstLvId, FirstLv>,
    #[serde(rename = "allItems", deserialize_with = "unique_map")]
    pub archives: IndexMap<ArchiveId, SourceArchive>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourcePage {
    pub page_type: Page,
    pub name: String,
    pub category_ids: Vec<Category>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourceCategory {
    pub category_id: Category,
    pub name: String,
    pub order: i64,
    pub r#type: Page,
    pub first_lv_ids: Vec<FirstLvId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FirstLv {
    pub first_lv_id: FirstLvId,
    pub category_id: Category,
    pub order: i64,
    pub r#type: Page,
    pub item_ids: Vec<ArchiveId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourceArchive {
    pub id: ArchiveId,
    pub first_lv_id: FirstLvId,
    pub category_id: Category,
    pub title: String,
    pub order: i64,
    pub r#type: Page,
}

#[derive(Deserialize)]
pub(super) struct Contract {
    pub version: u32,
    #[serde(deserialize_with = "unique_map")]
    pub categories: IndexMap<Category, Vec<ContractRow>>,
}

#[derive(Deserialize)]
pub(super) struct ContractRow {
    pub id: ArchiveId,
    pub acquisition: Acquisition,
}

#[derive(Deserialize)]
pub(super) struct Acquisition {
    pub method: AcquisitionMethod,
}

/// JSON object 重复键不能在反序列化时静默覆盖，否则后续无法发现身份冲突。
fn unique_map<'de, D, K, V>(deserializer: D) -> Result<IndexMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + Hash + fmt::Debug,
    V: Deserialize<'de>,
{
    struct UniqueMap<K, V>(PhantomData<(K, V)>);
    impl<'de, K, V> Visitor<'de> for UniqueMap<K, V>
    where
        K: Deserialize<'de> + Eq + Hash + fmt::Debug,
        V: Deserialize<'de>,
    {
        type Value = IndexMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("具有唯一键的 object")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut values = IndexMap::new();
            while let Some((key, value)) = map.next_entry()? {
                if values.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!("重复键: {key:?}")));
                }
                values.insert(key, value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueMap(PhantomData))
}
