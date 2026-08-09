//! `archive_acquisition_contract.json` 的 Rust 类型定义（与前端 `src/types/archiveAcquisitionContract.ts` 对齐）。
//!
//! 该数据文件描述档案库全部 462 个条目（对应 prts.json allItems 的 id）的获取方式。
//! 每条记录以 `method` 为判别字段，对应不同的获取途径与附加字段：
//!
//!   mission（完成任务）  → missionId / questId / logicId / special
//!   map（地图交互点位）  → logicId
//!   spec（特殊交互）     → special（levelId + logicId）
//!   auto（自动解锁）     → 无附加字段
//!   shop（商店兑换）     → shopGroupId / shopId / npcId / logicIds
//!   invstgt（研究提交）  → researchId
//!
//! 与 prts.rs 一致，采用「扁平结构 + Option 字段」表达按 method 条件出现的字段；
//! `special` 字段的两种形状用 [`Special`] 非标记（untagged）枚举区分。

use serde::{Deserialize, Serialize};

/// 档案获取方式（判别字段 method）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionMethod {
    /// 完成任务获得
    Mission,
    /// 地图交互点位获得
    Map,
    /// 特殊交互获得（如浮空回收器）
    Spec,
    /// 系统 / 百科自动解锁
    Auto,
    /// 商店兑换获得
    Shop,
    /// 研究提交后解锁
    Invstgt,
}

/// 任务类获取的对话特殊信息（`special`：NPC 对话 + Baker 消息链）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionSpecial {
    /// NPC id（如 suosi_map01）
    pub npc_id: String,
    /// NPC 对话选项 id（如 option_dlg_map01_lv002_env_8_1_001）
    pub dialog_option_id: String,
    /// Baker 消息链 id（如 sns_chat_nfm_0_1）
    pub baker_chat_id: String,
    /// Baker 对话 id（如 sns_f1m4d1_4）
    pub baker_dialog_id: String,
}

/// 特殊交互类获取的关卡节点信息（`special`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSpecial {
    /// 关卡 id（如 map01_lv001）
    pub level_id: String,
    /// 关卡内节点 logicId
    pub logic_id: i64,
}

/// `special` 字段（method = mission / spec 时出现，两种形状）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Special {
    /// 任务类：NPC 对话 + Baker 消息链
    Mission(MissionSpecial),
    /// 特殊交互类：关卡节点
    Spec(SpecSpecial),
}

/// 单条档案获取契约。
///
/// 扁平结构：按 `method` 条件出现的字段一律为 [`Option`]，
/// 是否有效由 [`AcquisitionMethod`] 决定（与前端判别联合对齐）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAcquisitionContractItem {
    /// 档案条目 id（对应 prts.json allItems 的 id）
    pub r#type: String,
    /// 获取方式
    pub method: AcquisitionMethod,
    /// 任务 id（method = mission 时）
    pub mission_id: Option<String>,
    /// 任务内子任务（quest）id（method = mission 时）
    pub quest_id: Option<String>,
    /// 地图 / 任务内节点 logicId（method = map / mission 时）
    pub logic_id: Option<i64>,
    /// 特殊信息（method = mission / spec 时）
    pub special: Option<Special>,
    /// 商店组 id（method = shop 时）
    pub shop_group_id: Option<String>,
    /// 商店页 id（method = shop 时）
    pub shop_id: Option<String>,
    /// 售卖 NPC id（method = shop 时）
    pub npc_id: Option<String>,
    /// 商店条目 logicId 列表（method = shop 时）
    pub logic_ids: Option<Vec<i64>>,
    /// 研究 id（method = invstgt 时）
    pub research_id: Option<String>,
}

pub type ArchiveAcquisitionContract = Vec<ArchiveAcquisitionContractItem>;

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实数据文件路径（相对 crate 根目录 src-tauri/）。
    const DATA_FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../resources/data/archive_acquisition_contract.json"
    );

    /// 读取真实数据文件并反序列化，校验全部条目都能被类型覆盖。
    #[test]
    fn deserialize_contracts() {
        let text = std::fs::read_to_string(DATA_FILE).expect("读取数据文件失败");
        let contracts: ArchiveAcquisitionContract =
            serde_json::from_str(&text).expect("反序列化失败");

        // 当前数据文件共 462 个配置条目（对应 PrtsAllItem 全部条目）
        assert_eq!(contracts.len(), 462);

        // 校验 method 与各变体必填字段的一致性
        for contract in &contracts {
            match contract.method {
                AcquisitionMethod::Mission => assert!(contract.mission_id.is_some()),
                AcquisitionMethod::Map => assert!(contract.logic_id.is_some()),
                AcquisitionMethod::Spec => assert!(contract.special.is_some()),
                AcquisitionMethod::Auto => {}
                AcquisitionMethod::Shop => {
                    assert!(contract.shop_group_id.is_some());
                    assert!(contract.shop_id.is_some());
                }
                AcquisitionMethod::Invstgt => assert!(contract.research_id.is_some()),
            }
        }
    }

    /// 序列化后重新反序列化，验证往返一致（含 untagged `special` 两种形状）。
    #[test]
    fn round_trip() {
        let text = std::fs::read_to_string(DATA_FILE).expect("读取数据文件失败");
        let contracts: ArchiveAcquisitionContract =
            serde_json::from_str(&text).expect("反序列化失败");

        let serialized = serde_json::to_string(&contracts).expect("序列化失败");
        let reparsed: ArchiveAcquisitionContract =
            serde_json::from_str(&serialized).expect("再次反序列化失败");

        assert_eq!(contracts.len(), reparsed.len());
    }
}
