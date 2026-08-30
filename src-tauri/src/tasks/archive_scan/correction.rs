//! 档案标题纠错：归一化 + 精确匹配 / 编辑距离评分。
//!
//! OCR 总归会有识别错误，本模块把 OCR 输出与该子分类下的候选标题（来自
//! `prts.json` 的 `allItems`）做比对，在置信度足够时纠出正确的标题。
//!
//! ## 错误形态
//!
//! 从已发现的 OCR 错误可归纳出三类：
//! 1. **字符替换**：形近字 / 简繁异体混淆（決→决）、全角半角标点（（→(、）→)）；
//! 2. **截断**（最主要）：OCR 区域只有一行，长标题只识别出第一行 → OCR 结果是正确标题的**前缀**；
//! 3. **（潜在）增删字符**：OCR 偶发多认 / 漏认字符。
//!
//! 由于「只识别标题第 1 行且第 1 行可唯一确定档案」，OCR 结果可建模为
//! **正确标题的前缀 + 少量字符错误**。候选池按子分类划分，每个分类仅 15~250 条，
//! 线性扫描即可，无需索引 / 机器学习。
//!
//! ## 算法
//!
//! 1. 离线构建候选索引（数据加载时一次完成）：按 `categoryId` 分组 + 归一化；
//! 2. 归一化 OCR 文本；
//! 3. 精确匹配（快速路径）：`O` 与某候选 `norm` 完全相等则直接采纳；
//! 4. 编辑距离评分：`score = 1 - dist / max(len(O), len(C.norm))`；
//! 5. 决策：`best ≥ 0.80` 且与次高差距 `≥ 0.10` → 纠错；否则「无法识别」。

use std::collections::HashMap;

use crate::types::PrtsData;

/// 归一化后参与比对的最大字符数（与「只识别标题第 1 行」的截断特征对齐）。
const NORM_MAX_CHARS: usize = 15;

/// 归一化置信度下限（低于此值不纠错）。
const SCORE_THRESHOLD: f64 = 0.80;
/// 最高分与次高分的差距下限（不足则不纠错，避免歧义）。
const SCORE_GAP_THRESHOLD: f64 = 0.10;

/// 一条候选档案（来自 `prts.json` 的 `allItems`）。
#[derive(Debug)]
struct Candidate {
    /// allItems 中的档案 id
    id: String,
    /// 原始标题（非归一化，采纳时返回原文）
    title: String,
    /// 归一化后的标题
    norm: String,
}

/// 纠错成功的结果。
#[derive(Debug, Clone)]
pub struct Corrected {
    /// 匹配到的档案原始标题（非归一化版）
    pub title: String,
    /// 匹配到的全部档案 id（同标题多条时返回全部）
    pub item_ids: Vec<String>,
}

/// 编辑距离评分后的一个候选组（同归一化标题的候选归为一组）。
struct Group {
    /// 原始标题（组内第一条的 title）
    title: String,
    /// 组内全部档案 id
    item_ids: Vec<String>,
    /// 相似度 `1 - dist / max(len(O), len(norm))`
    score: f64,
}

/// 纠错索引：`categoryId → 候选列表`。
///
/// 由 `prts.json` 构建，应用启动时加载一次，任务运行时只读共享。
#[derive(Debug, Default)]
pub struct CorrectionIndex {
    candidates: HashMap<String, Vec<Candidate>>,
}

impl CorrectionIndex {
    /// 从 `prts.json` 构建候选索引（按 `categoryId` 分组并归一化）。
    pub fn from_prts(prts: &PrtsData) -> Self {
        let mut candidates: HashMap<String, Vec<Candidate>> = HashMap::new();
        for item in prts.all_items.values() {
            candidates
                .entry(item.category_id.clone())
                .or_default()
                .push(Candidate {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    norm: normalize(&item.title),
                });
        }

        Self { candidates }
    }

    /// 索引中的候选总数（便于日志展示加载规模）。
    pub fn len(&self) -> usize {
        self.candidates.values().map(Vec::len).sum()
    }

    /// 索引是否为空（无候选时不纠错）。
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// 对当前子分类的 OCR 文本纠错。
    ///
    /// 返回 `None` 表示「无法识别」：没有高置信候选，不强行猜测。
    pub fn correct(&self, category_id: &str, ocr_text: &str) -> Option<Corrected> {
        let candidates = self.candidates.get(category_id)?;
        let o = normalize(ocr_text);
        if o.is_empty() {
            return None;
        }

        // 第 3 步：精确匹配（快速路径）
        let exact: Vec<&Candidate> = candidates.iter().filter(|c| c.norm == o).collect();
        if !exact.is_empty() {
            return Some(Corrected {
                title: exact[0].title.clone(),
                item_ids: exact.iter().map(|c| c.id.clone()).collect(),
            });
        }

        // 特判：digital 分类下 OCR 只识别到「文明」（掩码标题「■■■…文明■■■…保护协定」
        // 可见部分仅剩“文明”，归一化截断后常规算法无法匹配），直接纠错到该档案
        if category_id == "digital" && o == "文明" {
            if let Some(c) = candidates
                .iter()
                .find(|c| c.id == "nar_digital_map02_13003_1")
            {
                return Some(Corrected {
                    title: c.title.clone(),
                    item_ids: vec![c.id.clone()],
                });
            }
        }

        // 第 4 步：编辑距离评分，按归一化标题分组（同组距离相同，取组内全部 id）
        let o_len = o.chars().count();
        let mut groups: HashMap<&str, Group> = HashMap::new();
        for c in candidates {
            let dist = levenshtein(&o, &c.norm);
            let score = similarity(dist, o_len, c.norm.chars().count());
            let group = groups.entry(c.norm.as_str()).or_insert_with(|| Group {
                title: c.title.clone(),
                item_ids: Vec::new(),
                score,
            });
            group.item_ids.push(c.id.clone());
        }

        // 第 5 步：决策。按相似度降序，最高分与次高分差距不足则判「无法识别」。
        let mut groups: Vec<Group> = groups.into_values().collect();
        groups.sort_by(|a, b| b.score.total_cmp(&a.score));

        let best = &groups[0];
        if best.score < SCORE_THRESHOLD {
            return None;
        }
        // 只有一个候选组（整个分类只有一种标题）时无次高，直接采纳
        if groups.len() == 1 {
            return Some(best_to_corrected(best));
        }
        let second = &groups[1];
        if best.score - second.score >= SCORE_GAP_THRESHOLD {
            Some(best_to_corrected(best))
        } else {
            None
        }
    }
}

/// 把评分最高的候选组转为纠错结果。
fn best_to_corrected(best: &Group) -> Corrected {
    Corrected {
        title: best.title.clone(),
        item_ids: best.item_ids.clone(),
    }
}

/// 归一化 OCR 文本 / 候选标题（候选与 OCR 必须使用同一函数）。
///
/// 处理顺序：
/// 1. 去除富文本标签（`<@...>` 与 `</...>`，兼容全角尖括号）；
/// 2. 半角标点转全角（重点：(→（、)→） 等标点）；
/// 3. 取前 [`NORM_MAX_CHARS`] 个字符（与「只识别标题第 1 行」的截断特征对齐）；
/// 4. 替换映射表中的字符（決→决，空白符→空，方块字符 ■→空）。
pub fn normalize(text: &str) -> String {
    // 1. 去标签必须在转全角之前（标签使用半角尖括号）
    let no_tags = strip_rich_text_tags(text);
    // 2. 半角标点转全角
    let half_to_full: String = no_tags.chars().map(halfwidth_to_fullwidth).collect();
    // 3. 取前 N 个字符
    let truncated: String = half_to_full.chars().take(NORM_MAX_CHARS).collect();
    // 4. 替换映射（含删除空白符 / ■）
    truncated
        .chars()
        .filter_map(|c| {
            if is_blank(c) || c == '■' {
                None
            } else {
                Some(replace_char(c))
            }
        })
        .collect()
}

/// 字符替换映射表（形近字 / 简繁异体混淆，可扩展）。
const CHAR_REPLACE: &[(char, char)] = &[('決', '决')];

/// 查替换映射表；未命中时返回原字符。
fn replace_char(c: char) -> char {
    CHAR_REPLACE
        .iter()
        .find(|(from, _)| *from == c)
        .map(|(_, to)| *to)
        .unwrap_or(c)
}

/// 去除富文本标签：跳过 `<...>` 与 `</...>`（兼容全角尖括号）。
fn strip_rich_text_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' || c == '＜' {
            in_tag = true;
            continue;
        }
        if c == '>' || c == '＞' {
            in_tag = false;
            continue;
        }
        if !in_tag {
            out.push(c);
        }
    }
    out
}

/// 半角标点转全角（仅标点：U+0021–U+002F、U+003A–U+0040、U+005B–U+0060、U+007B–U+007E）。
/// 字母、数字不转换（候选标题中的半角字母 / 数字按原样保留）。
fn halfwidth_to_fullwidth(c: char) -> char {
    let code = c as u32;
    let is_punctuation = matches!(
        code,
        0x21..=0x2F | 0x3A..=0x40 | 0x5B..=0x60 | 0x7B..=0x7E
    );
    if is_punctuation {
        char::from_u32(code + 0xFEE0).unwrap_or(c)
    } else {
        c
    }
}

/// 是否为空白字符（含全角空格 U+3000、零宽空格 U+200B）。
fn is_blank(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{200b}' | '\u{3000}')
}

/// 计算两个字符串的 Levenshtein 编辑距离（按 Unicode 字符计）。
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            curr[j + 1] = if ca == cb {
                prev[j]
            } else {
                (prev[j] + 1).min(curr[j] + 1).min(prev[j + 1] + 1)
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// 相似度：`1 - dist / max(len(a), len(b))`。
fn similarity(dist: usize, a_len: usize, b_len: usize) -> f64 {
    let max_len = a_len.max(b_len);
    if max_len == 0 {
        return 1.0;
    }
    1.0 - dist as f64 / max_len as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 构造一个覆盖设计文档错误案例的迷你索引。
    ///
    /// `allItems` 条目为 [`PrtsAllItem`] 的最小合法 JSON（含 `type` / `firstLvId` /
    /// `name` / `order` 等必填字段），反序列化后验证纠错逻辑。
    fn test_index() -> CorrectionIndex {
        let prts = json!({
            // 纠错只用到 allItems，其余三个部分可为空
            "PrtsPage": {},
            "PrtsCategory": {},
            "firstLv": {},
            "allItems": {
                "nar_media_map01_108_1": {
                    "id": "nar_media_map01_108_1",
                    "title": "决然工人的留声",
                    "name": "决然工人的留声",
                    "categoryId": "media",
                    "firstLvId": "media_1",
                    "order": 1,
                    "type": "multi_media"
                },
                "nar_paper_map01_122_1": {
                    "id": "nar_paper_map01_122_1",
                    "title": "工团大会预算申报宣讲草稿（第八版）",
                    "name": "工团大会预算申报宣讲草稿（第八版）",
                    "categoryId": "paper",
                    "firstLvId": "paper_1",
                    "order": 1,
                    "type": "text"
                },
                "nar_paper_map01_110_1": {
                    "id": "nar_paper_map01_110_1",
                    "title": "《味蕾上的四号谷地：工团杂烩汤篇》",
                    "name": "《味蕾上的四号谷地：工团杂烩汤篇》",
                    "categoryId": "paper",
                    "firstLvId": "paper_2",
                    "order": 1,
                    "type": "text"
                },
                "nar_paper_map01_116_1": {
                    "id": "nar_paper_map01_116_1",
                    "title": "《四号谷地工作指南：阿伯莉采石场篇》",
                    "name": "《四号谷地工作指南：阿伯莉采石场篇》",
                    "categoryId": "paper",
                    "firstLvId": "paper_3",
                    "order": 1,
                    "type": "text"
                },
                "nar_report_map01_research2_4_1": {
                    "id": "nar_report_map01_research2_4_1",
                    "title": "裂地者控制区内疑似工团成员的信号分析",
                    "name": "裂地者控制区内疑似工团成员的信号分析",
                    "categoryId": "paper",
                    "firstLvId": "paper_4",
                    "order": 1,
                    "type": "text"
                },
                "nar_paper_map01_59_1": {
                    "id": "nar_paper_map01_59_1",
                    "title": "天空观测记录（四号谷地）",
                    "name": "天空观测记录（四号谷地）",
                    "categoryId": "paper",
                    "firstLvId": "paper_5",
                    "order": 1,
                    "type": "text"
                },
                // 与「天空观测记录（四号谷地）」仅一字之差，用于歧义测试
                "nar_paper_map01_59_2": {
                    "id": "nar_paper_map01_59_2",
                    "title": "天空观测记录（五号谷地）",
                    "name": "天空观测记录（五号谷地）",
                    "categoryId": "paper",
                    "firstLvId": "paper_5",
                    "order": 2,
                    "type": "text"
                },
                // 同标题多条（挂在竹子上的字条 ×2）
                "nar_dup_1": {
                    "id": "nar_dup_1",
                    "title": "挂在竹子上的字条",
                    "name": "挂在竹子上的字条",
                    "categoryId": "digital",
                    "firstLvId": "digital_1",
                    "order": 1,
                    "type": "text"
                },
                "nar_dup_2": {
                    "id": "nar_dup_2",
                    "title": "挂在竹子上的字条",
                    "name": "挂在竹子上的字条",
                    "categoryId": "digital",
                    "firstLvId": "digital_1",
                    "order": 2,
                    "type": "text"
                }
            }
        });
        let prts: PrtsData = serde_json::from_value(prts).expect("测试数据解析失败");
        CorrectionIndex::from_prts(&prts)
    }

    #[test]
    fn normalize_replaces_traditional_chars() {
        assert_eq!(normalize("決然工人的留声"), normalize("决然工人的留声"));
    }

    #[test]
    fn normalize_converts_halfwidth_parens() {
        assert_eq!(normalize("（第八版）"), normalize("(第八版)"));
    }

    #[test]
    fn normalize_strips_rich_text_tags_and_blocks() {
        // 富文本标签 / ■ / 零宽空格 / 全角空格都应被剥离
        assert_eq!(normalize("<@nar.mark>\u{200b}■■</>文明"), normalize("文明"));
        assert_eq!(normalize("A\u{3000}B"), normalize("AB"));
    }

    #[test]
    fn normalize_truncates_to_15_chars() {
        assert_eq!(
            normalize("裂地者控制区内疑似工团成员的信号分析"),
            normalize("裂地者控制区内疑似工团成员的信号")
        );
    }

    #[test]
    fn correct_character_replacement() {
        let idx = test_index();
        let c = idx.correct("media", "決然工人的留声").expect("应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
        assert_eq!(c.item_ids, vec!["nar_media_map01_108_1"]);
    }

    #[test]
    fn correct_truncated_title_via_exact_match() {
        let idx = test_index();
        let c = idx
            .correct("paper", "工团大会预算申报宣讲草稿（第八")
            .expect("截断标题应通过归一化精确匹配");
        assert_eq!(c.title, "工团大会预算申报宣讲草稿（第八版）");
        assert_eq!(c.item_ids, vec!["nar_paper_map01_122_1"]);
    }

    #[test]
    fn correct_truncated_book_title() {
        let idx = test_index();
        let c = idx
            .correct("paper", "《味蕾上的四号谷地：工团杂烩汤")
            .expect("截断的书名应纠错成功");
        assert_eq!(c.title, "《味蕾上的四号谷地：工团杂烩汤篇》");
    }

    #[test]
    fn correct_editing_distance_within_gap() {
        let idx = test_index();
        // OCR 错一个字（声→生），且该分类只有这一个高置信候选
        let c = idx
            .correct("media", "决然工人的留生")
            .expect("编辑距离相近应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
    }

    #[test]
    fn correct_returns_all_ids_for_duplicate_titles() {
        let idx = test_index();
        let c = idx
            .correct("digital", "挂在竹子上的字条")
            .expect("同标题多条应全部命中");
        assert_eq!(c.item_ids.len(), 2);
        assert!(c.item_ids.contains(&"nar_dup_1".to_string()));
        assert!(c.item_ids.contains(&"nar_dup_2".to_string()));
    }

    #[test]
    fn correct_ambiguous_returns_none() {
        let idx = test_index();
        // OCR 少一个右括号：与「四号谷地」差 1 步（0.909）、与「五号谷地」差 2 步（0.818），
        // 分差不足 0.10，不应强行纠错
        assert!(idx.correct("paper", "天空观测记录（四号谷地").is_none());
    }

    #[test]
    fn correct_unknown_category_returns_none() {
        let idx = test_index();
        assert!(idx.correct("no_such_category", "决然工人的留声").is_none());
    }

    #[test]
    fn correct_empty_ocr_returns_none() {
        let idx = test_index();
        assert!(idx.correct("media", "").is_none());
    }

    /// 全量验证：加载真实 prts.json，把每个标题模拟成「截断 / 替换」后的 OCR 输出，
    /// 检查纠错是否命中原条目（标题一致即可，同标题多条允许命中任一）。
    #[test]
    fn correct_all_real_titles() {
        // cargo test 的工作目录是 src-tauri/
        let prts_path = std::path::Path::new("../resources/data/prts.json");
        if !prts_path.exists() {
            eprintln!("跳过：未找到真实 prts.json（{}）", prts_path.display());
            return;
        }
        let text = std::fs::read_to_string(prts_path).expect("读取 prts.json 失败");
        let prts: PrtsData = serde_json::from_str(&text).expect("解析 prts.json 失败");
        let idx = CorrectionIndex::from_prts(&prts);

        let mut total = 0usize;
        let mut hit = 0usize;
        let mut miss: Vec<(String, String, String)> = Vec::new();

        for item in prts.all_items.values() {
            let id = &item.id;
            let title = &item.title;
            let category_id = &item.category_id;
            let normalized_title = normalize(title);
            if normalized_title.is_empty() {
                // 打码标题归一化为空，无法纠错，跳过
                continue;
            }

            // 模拟 OCR：只识别标题第 1 行。短标题（≤15 字符）在一行内完整显示 → 完整识别；
            // 长标题只识别到前 15 字符（与归一化截断对齐）。再混入一个形近字。
            let chars: Vec<char> = title.chars().collect();
            let mut ocr: Vec<char> = if chars.len() > NORM_MAX_CHARS {
                chars.into_iter().take(NORM_MAX_CHARS).collect()
            } else {
                chars
            };
            if let Some(first) = ocr.first_mut() {
                if *first == '决' {
                    *first = '決';
                }
            }
            let ocr: String = ocr.into_iter().collect();

            total += 1;
            match idx.correct(category_id, &ocr) {
                Some(c) if c.item_ids.iter().any(|i| i == id) => hit += 1,
                Some(c) => miss.push((id.clone(), title.clone(), c.title)),
                None => miss.push((id.clone(), title.clone(), String::new())),
            }
        }

        eprintln!(
            "全量纠错验证：{}/{} 命中（{:.1}%），{} 个未命中",
            hit,
            total,
            hit as f64 * 100.0 / total as f64,
            miss.len()
        );
        for (id, title, got) in miss.iter().take(20) {
            eprintln!("  未命中: {id} {title:?} -> {got:?}");
        }
        // 允许少量边界情况（如同前缀歧义），但不允许超过 5% 失败
        assert!(
            miss.len() <= total / 20,
            "未命中过多: {}/{}",
            miss.len(),
            total
        );
    }
}
