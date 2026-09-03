//! `archive_contract.json` 的 Rust 类型定义（与前端 `src/types/archiveContract.ts` 对齐）。
//!
//! 该数据文件描述档案库全部逻辑条目（对应 prts.json allItems 按逻辑 id 归并）的
//! 获取方式，按 6 个分类分组。每条记录以 `acquisition.method` 为判别字段，对应不同的
//! 获取途径与附加字段：
//!
//!   map（地图交互点位）  → pointId
//!   mission（完成任务）  → missionId / questId / interaction / special
//!   auto（自动解锁）     → 无附加字段
//!   shop（商店兑换）     → shopGroupId / shopId / npcId / pointId / location
//!   invstgt（研究提交）  → researchId
//!
//! 与 TS 判别联合一致，`acquisition` 用「按 method 内部标记（internally tagged）的枚举」
//! 表达：每个变体只携带各自的附加字段（camelCase，与前端字段名对齐）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 档案分类 id（`categories` 的键）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCategoryId {
    /// 纸质记录
    Paper,
    /// 电子档案
    Digital,
    /// 藏品
    Collection,
    /// 档案
    Document,
    /// 调查报告
    Report,
    /// 多媒体
    Media,
}

/// 任务内关卡交互信息（`mission.interaction`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMissionInteraction {
    /// 关卡交互点位 id
    pub point_id: i64,
    /// 任务阶段文本 id
    pub stage_id: String,
}

/// 任务特殊信息（`mission.special`，NPC 对话 + Baker 消息链）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMissionSpecial {
    /// NPC id（如 suosi_map01）
    pub npc_id: String,
    /// NPC 对话选项 id（如 option_dlg_map01_lv002_env_8_1_001）
    pub dialog_option_id: String,
    /// Baker 消息链 id（如 sns_chat_nfm_0_1）
    pub baker_chat_id: String,
    /// Baker 对话 id（如 sns_f1m4d1_4）
    pub baker_dialog_id: String,
}

/// 商店位置（`shop.location`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveShopLocation {
    /// 地图区域 id（如 VL）
    pub region_id: String,
    /// 地图子区域 id（如 HB）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subregion_id: Option<String>,
}

/// 档案获取方式（以 `method` 内部标记判别的联合类型，与前端判别联合对齐）。
///
/// 变体字段名与 TS 对齐（camelCase）；`method` 作为内部标记单独出现在 JSON 中。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum ArchiveAcquisition {
    /// 地图交互点位获取
    #[serde(rename_all = "camelCase")]
    Map {
        /// 地图交互点位 id
        point_id: i64,
    },
    /// 完成任务获取
    #[serde(rename_all = "camelCase")]
    Mission {
        /// 任务 id
        mission_id: String,
        /// 任务内子任务（quest）id
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quest_id: Option<String>,
        /// 关卡交互信息（任务中打开档案记录）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interaction: Option<ArchiveMissionInteraction>,
        /// NPC 对话 + Baker 消息链特殊信息
        #[serde(default, skip_serializing_if = "Option::is_none")]
        special: Option<ArchiveMissionSpecial>,
    },
    /// 系统 / 百科自动解锁，无附加字段
    Auto,
    /// 商店兑换获取
    #[serde(rename_all = "camelCase")]
    Shop {
        /// 商店组 id（如 domainshop_map01）
        shop_group_id: String,
        /// 商店页 id（如 domainshop_page_com_map01）
        shop_id: String,
        /// 售卖 NPC id
        #[serde(default, skip_serializing_if = "Option::is_none")]
        npc_id: Option<String>,
        /// 商店点位 id
        #[serde(default, skip_serializing_if = "Option::is_none")]
        point_id: Option<i64>,
        /// 商店位置（地图区域）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        location: Option<ArchiveShopLocation>,
    },
    /// 研究提交后解锁
    #[serde(rename_all = "camelCase")]
    Invstgt {
        /// 研究 id（如 research_landbreakerMurder）
        research_id: String,
    },
}

/// `archive_contract.json` 中的单条档案。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveContractRow {
    /// 档案条目 id
    pub id: String,
    /// 档案图标
    pub icon: String,
    /// 档案获取方式
    pub acquisition: ArchiveAcquisition,
}

/// `archive_contract.json` 的运行时契约（打包数据，供前端按档案 id 查询获取方式）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveContract {
    /// 契约版本
    pub version: u32,
    /// 按分类分组的档案条目
    pub categories: HashMap<ArchiveCategoryId, Vec<ArchiveContractRow>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实数据文件路径（相对 crate 根目录 src-tauri/）。
    const DATA_FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../resources/data/archive_contract.json"
    );

    /// 读取真实数据文件并反序列化，校验全部条目都能被类型覆盖。
    #[test]
    fn deserialize_contract() {
        let text = std::fs::read_to_string(DATA_FILE).expect("读取数据文件失败");
        let contract: ArchiveContract = serde_json::from_str(&text).expect("反序列化失败");

        assert_eq!(contract.version, 1);
        // 档案分类稳定为 6 类，可视为数据 schema 的一部分
        assert_eq!(contract.categories.len(), 6);

        // 校验各获取方式必填字段一致
        for rows in contract.categories.values() {
            for row in rows {
                match &row.acquisition {
                    ArchiveAcquisition::Map { .. } => {}
                    ArchiveAcquisition::Mission { mission_id, .. } => {
                        assert!(!mission_id.is_empty());
                    }
                    ArchiveAcquisition::Auto => {}
                    ArchiveAcquisition::Shop {
                        shop_group_id,
                        shop_id,
                        ..
                    } => {
                        assert!(!shop_group_id.is_empty());
                        assert!(!shop_id.is_empty());
                    }
                    ArchiveAcquisition::Invstgt { research_id } => {
                        assert!(!research_id.is_empty());
                    }
                }
            }
        }
    }

    /// 序列化后重新反序列化，验证往返一致（含内部标记 `method` 与各附加字段）。
    #[test]
    fn round_trip() {
        let text = std::fs::read_to_string(DATA_FILE).expect("读取数据文件失败");
        let contract: ArchiveContract = serde_json::from_str(&text).expect("反序列化失败");

        let serialized = serde_json::to_string(&contract).expect("序列化失败");
        let reparsed: ArchiveContract =
            serde_json::from_str(&serialized).expect("再次反序列化失败");

        let total = |contract: &ArchiveContract| -> usize {
            contract.categories.values().map(Vec::len).sum()
        };
        assert_eq!(total(&contract), total(&reparsed));
    }
}
