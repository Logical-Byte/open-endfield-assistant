//! 档案标题纠错。
//!
//! 使用候选标题索引和可选覆盖项，把原始 OCR 文本解析为档案条目。

use crate::data::{
    ArchiveTitleIndex,
    archive_title_index::{Candidate, normalize},
};

/// 归一化置信度下限（低于此值不纠错）。
const SCORE_THRESHOLD: f64 = 0.80;
/// 最高分与次高分的差距下限（不足则不纠错，避免歧义）。
const SCORE_GAP_THRESHOLD: f64 = 0.10;

/// 无法由常规标题匹配处理的已知 OCR 结果。
#[derive(Debug, Clone, Copy)]
pub struct CorrectionOverride<'a> {
    category_id: &'a str,
    observed_text: &'a str,
    item_id: &'a str,
}

impl<'a> CorrectionOverride<'a> {
    pub const fn new(category_id: &'a str, observed_text: &'a str, item_id: &'a str) -> Self {
        Self {
            category_id,
            observed_text,
            item_id,
        }
    }
}

/// 档案扫描任务默认启用的纠错覆盖项。
pub(super) const DEFAULT_CORRECTION_OVERRIDES: &[CorrectionOverride<'static>] =
    &[CorrectionOverride::new(
        "digital",
        "文明",
        "nar_digital_map02_13003_1",
    )];

/// 纠错成功的结果。
#[derive(Debug, Clone)]
pub struct Corrected {
    /// 匹配到的档案原始标题（非归一化版）
    pub title: String,
    /// 匹配到的全部档案 id（同标题多条时返回全部）
    pub item_ids: Vec<String>,
}

/// 编辑距离评分后的候选组。
struct ScoredGroup<'a> {
    candidates: &'a [Candidate],
    /// 相似度 `1 - dist / max(len(O), len(norm))`
    score: f64,
}

/// 在指定子分类中把原始 OCR 文本纠正为档案标题与条目 ID。
///
/// `ocr_text` 会先通过 `normalize` 转换为索引使用的形式。规范化结果为空时返回
/// `None`。`overrides` 为 `None` 时等价于空列表；覆盖项中的 `observed_text` 使用同一
/// 规范化规则后再参与匹配。
///
/// # 匹配顺序
///
/// 1. 通过 `ArchiveTitleIndex::by_normalized_title` 查找规范化标题完全相同的候选组；
/// 2. 按列表顺序查找分类与观察文本都匹配的覆盖项，再通过条目 ID 读取目标候选；
/// 3. 对该分类的每个规范化标题计算 Unicode 字符级 Levenshtein 编辑距离，相似度为
///    `1 - distance / max(ocr_length, title_length)`；
/// 4. 最高相似度不低于 `SCORE_THRESHOLD`，且只有一个候选组或与次高分的差距不低于
///    `SCORE_GAP_THRESHOLD` 时采用最高分候选组。
///
/// 精确匹配先于覆盖项，因此覆盖项不会替换已经命中的规范化标题。覆盖项引用的条目
/// 不存在时继续尝试编辑距离匹配。
///
/// # 返回值
///
/// 索引匹配命中后返回候选组第一项的原始标题，以及该规范化标题下的全部条目 ID；覆盖项
/// 命中后只返回它指定的条目。未找到候选、分数不足或最高分存在歧义时返回 `None`。
pub fn correct(
    index: &ArchiveTitleIndex,
    category_id: &str,
    ocr_text: &str,
    overrides: Option<&[CorrectionOverride<'_>]>,
) -> Option<Corrected> {
    let overrides = overrides.unwrap_or(&[]);
    let normalized_ocr = normalize(ocr_text);
    if normalized_ocr.is_empty() {
        return None;
    }

    // 第 3 步：精确匹配（快速路径）
    if let Some(matching_candidates) = index.by_normalized_title(category_id, &normalized_ocr) {
        return Some(to_corrected(matching_candidates));
    }

    // 第 4 步：覆盖项匹配
    if let Some(corrected) = apply_override(index, overrides, category_id, &normalized_ocr) {
        return Some(corrected);
    }

    // 第 5 步：编辑距离评分
    let normalized_ocr_len = normalized_ocr.chars().count();
    let mut scored_groups: Vec<ScoredGroup<'_>> = index
        .normalized_groups(category_id)
        .map(|(normalized_title, candidates)| {
            let edit_distance = levenshtein(&normalized_ocr, normalized_title);
            ScoredGroup {
                candidates,
                score: similarity(
                    edit_distance,
                    normalized_ocr_len,
                    normalized_title.chars().count(),
                ),
            }
        })
        .collect();

    // 第 6 步：决策。按相似度降序，最高分与次高分差距不足则判「无法识别」。
    scored_groups.sort_by(|left, right| right.score.total_cmp(&left.score));

    let best_group = scored_groups.first()?;
    if best_group.score < SCORE_THRESHOLD {
        return None;
    }
    // 只有一个候选组（整个分类只有一种标题）时无次高，直接采纳
    if scored_groups.len() == 1 {
        return Some(to_corrected(best_group.candidates));
    }
    let second_best_group = &scored_groups[1];
    if best_group.score - second_best_group.score >= SCORE_GAP_THRESHOLD {
        Some(to_corrected(best_group.candidates))
    } else {
        None
    }
}

fn apply_override(
    index: &ArchiveTitleIndex,
    overrides: &[CorrectionOverride<'_>],
    category_id: &str,
    normalized_ocr: &str,
) -> Option<Corrected> {
    overrides
        .iter()
        .find(|correction_override| {
            correction_override.category_id == category_id
                && normalize(correction_override.observed_text) == normalized_ocr
        })
        .and_then(|correction_override| {
            index.candidate_by_id(category_id, correction_override.item_id)
        })
        .map(|candidate| to_corrected(std::slice::from_ref(candidate)))
}

/// 把候选组转为纠错结果。
fn to_corrected(candidates: &[Candidate]) -> Corrected {
    Corrected {
        title: candidates[0].title().to_string(),
        item_ids: candidates
            .iter()
            .map(|candidate| candidate.id().to_string())
            .collect(),
    }
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
    use crate::data::{PrtsData, archive_title_index::NORM_MAX_CHARS};
    use serde_json::json;

    /// 构造一个覆盖设计文档错误案例的迷你索引。
    ///
    /// `allItems` 条目为 [`PrtsAllItem`] 的最小合法 JSON（含 `type` / `firstLvId` /
    /// `name` / `order` 等必填字段），反序列化后验证纠错逻辑。
    fn test_index() -> ArchiveTitleIndex {
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
                "nar_digital_map02_13003_1": {
                    "id": "nar_digital_map02_13003_1",
                    "title": "■■■■■■■■■■■■■■■文明■■■■保护协定",
                    "name": "■■■■■■■■■■■■■■■文明■■■■保护协定",
                    "categoryId": "digital",
                    "firstLvId": "digital_1",
                    "order": 1,
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
        ArchiveTitleIndex::from_prts(&prts)
    }

    #[test]
    fn correct_character_replacement() {
        let idx = test_index();
        let c = correct(&idx, "media", "決然工人的留声", None).expect("应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
        assert_eq!(c.item_ids, vec!["nar_media_map01_108_1"]);
    }

    #[test]
    fn correct_truncated_title_via_exact_match() {
        let idx = test_index();
        let c = correct(&idx, "paper", "工团大会预算申报宣讲草稿（第八", None)
            .expect("截断标题应通过归一化精确匹配");
        assert_eq!(c.title, "工团大会预算申报宣讲草稿（第八版）");
        assert_eq!(c.item_ids, vec!["nar_paper_map01_122_1"]);
    }

    #[test]
    fn correct_truncated_book_title() {
        let idx = test_index();
        let c = correct(&idx, "paper", "《味蕾上的四号谷地：工团杂烩汤", None)
            .expect("截断的书名应纠错成功");
        assert_eq!(c.title, "《味蕾上的四号谷地：工团杂烩汤篇》");
    }

    #[test]
    fn correct_editing_distance_within_gap() {
        let idx = test_index();
        // OCR 错一个字（声→生），且该分类只有这一个高置信候选
        let c = correct(&idx, "media", "决然工人的留生", None).expect("编辑距离相近应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
    }

    #[test]
    fn correct_returns_all_ids_for_duplicate_titles() {
        let idx = test_index();
        let c = correct(&idx, "digital", "挂在竹子上的字条", None).expect("同标题多条应全部命中");
        assert_eq!(c.item_ids.len(), 2);
        assert!(c.item_ids.contains(&"nar_dup_1".to_string()));
        assert!(c.item_ids.contains(&"nar_dup_2".to_string()));
    }

    #[test]
    fn correct_fuzzy_match_returns_all_ids_for_duplicate_titles() {
        let idx = test_index();
        let c =
            correct(&idx, "digital", "挂在竹子上的纸条", None).expect("模糊匹配同标题时应全部命中");
        assert_eq!(c.item_ids, vec!["nar_dup_1", "nar_dup_2"]);
    }

    #[test]
    fn correct_ambiguous_returns_none() {
        let idx = test_index();
        // OCR 少一个右括号：与「四号谷地」差 1 步（0.909）、与「五号谷地」差 2 步（0.818），
        // 分差不足 0.10，不应强行纠错
        assert!(correct(&idx, "paper", "天空观测记录（四号谷地", None).is_none());
    }

    #[test]
    fn correct_unknown_category_returns_none() {
        let idx = test_index();
        assert!(correct(&idx, "no_such_category", "决然工人的留声", None).is_none());
    }

    #[test]
    fn correct_empty_ocr_returns_none() {
        let idx = test_index();
        assert!(correct(&idx, "media", "", None).is_none());
    }

    #[test]
    fn correct_uses_injected_override() {
        let idx = test_index();

        assert!(correct(&idx, "digital", "文明", None).is_none());

        let corrected = correct(&idx, "digital", "文明", Some(DEFAULT_CORRECTION_OVERRIDES))
            .expect("覆盖项应指定对应档案");
        assert_eq!(corrected.item_ids, vec!["nar_digital_map02_13003_1"]);
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
        let idx = ArchiveTitleIndex::from_prts(&prts);

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
            match correct(&idx, category_id, &ocr, Some(DEFAULT_CORRECTION_OVERRIDES)) {
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
