//! 档案标题纠错。
//!
//! 使用候选标题索引和可选覆盖项，把原始 OCR 文本解析为档案条目。

use crate::data::archive::{self, normalize};
use std::sync::LazyLock;

/// 归一化置信度下限（低于此值不纠错）。
const SCORE_THRESHOLD: f64 = 0.80;
/// 最高分与次高分的差距下限（不足则不纠错，避免歧义）。
const SCORE_GAP_THRESHOLD: f64 = 0.10;

/// 无法由常规标题匹配处理的已知 OCR 结果。
#[derive(Debug, Clone)]
pub struct CorrectionOverride<'a> {
    category_id: archive::Category,
    observed_text: &'a str,
    item_id: archive::ArchiveId,
}

impl<'a> CorrectionOverride<'a> {
    pub fn new(
        category_id: archive::Category,
        observed_text: &'a str,
        item_id: archive::ArchiveId,
    ) -> Self {
        Self {
            category_id,
            observed_text,
            item_id,
        }
    }
}

/// 档案扫描任务默认启用的纠错覆盖项。
pub(super) static DEFAULT_CORRECTION_OVERRIDES: LazyLock<Vec<CorrectionOverride<'static>>> =
    LazyLock::new(|| {
        vec![CorrectionOverride::new(
            archive::Category::Digital,
            "文明",
            archive::ArchiveId::new("nar_digital_map02_13003_1"),
        )]
    });

/// 纠错成功的结果。
#[derive(Debug, Clone)]
pub struct Corrected {
    /// 匹配到的档案原始标题（非归一化版）
    pub title: String,
    /// 匹配到的全部档案 id（同标题多条时返回全部）
    pub item_ids: Vec<archive::ArchiveId>,
}

/// 编辑距离评分后的候选组。
struct ScoredGroup<'a> {
    candidates: archive::Candidates<'a>,
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
/// 1. 通过 `Database::by_normalized_title` 查找规范化标题完全相同的候选组；
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
pub fn match_with_correction(
    index: &archive::Database,
    category_id: archive::Category,
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
        return to_corrected(matching_candidates);
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
        return to_corrected(best_group.candidates);
    }
    let second_best_group = &scored_groups[1];
    if best_group.score - second_best_group.score >= SCORE_GAP_THRESHOLD {
        to_corrected(best_group.candidates)
    } else {
        None
    }
}

fn apply_override(
    index: &archive::Database,
    overrides: &[CorrectionOverride<'_>],
    category_id: archive::Category,
    normalized_ocr: &str,
) -> Option<Corrected> {
    overrides
        .iter()
        .find(|correction_override| {
            correction_override.category_id == category_id
                && normalize(correction_override.observed_text) == normalized_ocr
        })
        .and_then(|correction_override| {
            index.candidate_by_id(category_id, &correction_override.item_id)
        })
        .map(|candidate| Corrected {
            title: candidate.title.clone(),
            item_ids: vec![candidate.id.clone()],
        })
}

/// 把候选组转为纠错结果。
fn to_corrected(candidates: archive::Candidates<'_>) -> Option<Corrected> {
    let title = candidates.iter().next()?.title.clone();
    Some(Corrected {
        title,
        item_ids: candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect(),
    })
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
    use crate::{app_paths::AppPaths, data::archive::NORM_MAX_CHARS};

    fn test_index() -> archive::Database {
        let rows = [
            (
                "nar_media_map01_108_1",
                archive::Category::Media,
                "决然工人的留声",
            ),
            (
                "nar_paper_map01_122_1",
                archive::Category::Paper,
                "工团大会预算申报宣讲草稿（第八版）",
            ),
            (
                "nar_paper_map01_110_1",
                archive::Category::Paper,
                "《味蕾上的四号谷地：工团杂烩汤篇》",
            ),
            (
                "nar_paper_map01_116_1",
                archive::Category::Paper,
                "《四号谷地工作指南：阿伯莉采石场篇》",
            ),
            (
                "nar_report_map01_research2_4_1",
                archive::Category::Paper,
                "裂地者控制区内疑似工团成员的信号分析",
            ),
            (
                "nar_paper_map01_59_1",
                archive::Category::Paper,
                "天空观测记录（四号谷地）",
            ),
            (
                "nar_paper_map01_59_2",
                archive::Category::Paper,
                "天空观测记录（五号谷地）",
            ),
            (
                "nar_digital_map02_13003_1",
                archive::Category::Digital,
                "■■■■■■■■■■■■■■■文明■■■■保护协定",
            ),
            ("nar_dup_1", archive::Category::Digital, "挂在竹子上的字条"),
            ("nar_dup_2", archive::Category::Digital, "挂在竹子上的字条"),
        ];
        archive::Database::from_catalog(archive::Catalog {
            pages: Vec::new(),
            categories: Vec::new(),
            archives: rows
                .into_iter()
                .map(|(id, category, title)| archive::ArchiveEntry {
                    id: archive::ArchiveId::new(id),
                    category,
                    title: title.to_string(),
                    acquisition_method: archive::AcquisitionMethod::Auto,
                })
                .collect(),
        })
    }

    #[test]
    fn correct_character_replacement() {
        let idx = test_index();
        let c = match_with_correction(&idx, archive::Category::Media, "決然工人的留声", None)
            .expect("应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
        assert_eq!(
            c.item_ids,
            vec![archive::ArchiveId::new("nar_media_map01_108_1")]
        );
    }

    #[test]
    fn correct_truncated_title_via_exact_match() {
        let idx = test_index();
        let c = match_with_correction(
            &idx,
            archive::Category::Paper,
            "工团大会预算申报宣讲草稿（第八",
            None,
        )
        .expect("截断标题应通过归一化精确匹配");
        assert_eq!(c.title, "工团大会预算申报宣讲草稿（第八版）");
        assert_eq!(
            c.item_ids,
            vec![archive::ArchiveId::new("nar_paper_map01_122_1")]
        );
    }

    #[test]
    fn correct_truncated_book_title() {
        let idx = test_index();
        let c = match_with_correction(
            &idx,
            archive::Category::Paper,
            "《味蕾上的四号谷地：工团杂烩汤",
            None,
        )
        .expect("截断的书名应纠错成功");
        assert_eq!(c.title, "《味蕾上的四号谷地：工团杂烩汤篇》");
    }

    #[test]
    fn correct_editing_distance_within_gap() {
        let idx = test_index();
        // OCR 错一个字（声→生），且该分类只有这一个高置信候选
        let c = match_with_correction(&idx, archive::Category::Media, "决然工人的留生", None)
            .expect("编辑距离相近应纠错成功");
        assert_eq!(c.title, "决然工人的留声");
    }

    #[test]
    fn correct_returns_all_ids_for_duplicate_titles() {
        let idx = test_index();
        let c = match_with_correction(&idx, archive::Category::Digital, "挂在竹子上的字条", None)
            .expect("同标题多条应全部命中");
        assert_eq!(c.item_ids.len(), 2);
        assert!(c.item_ids.contains(&archive::ArchiveId::new("nar_dup_1")));
        assert!(c.item_ids.contains(&archive::ArchiveId::new("nar_dup_2")));
    }

    #[test]
    fn correct_fuzzy_match_returns_all_ids_for_duplicate_titles() {
        let idx = test_index();
        let c = match_with_correction(&idx, archive::Category::Digital, "挂在竹子上的纸条", None)
            .expect("模糊匹配同标题时应全部命中");
        assert_eq!(
            c.item_ids,
            vec![
                archive::ArchiveId::new("nar_dup_1"),
                archive::ArchiveId::new("nar_dup_2")
            ]
        );
    }

    #[test]
    fn correct_ambiguous_returns_none() {
        let idx = test_index();
        // OCR 少一个右括号：与「四号谷地」差 1 步（0.909）、与「五号谷地」差 2 步（0.818），
        // 分差不足 0.10，不应强行纠错
        assert!(
            match_with_correction(
                &idx,
                archive::Category::Paper,
                "天空观测记录（四号谷地",
                None
            )
            .is_none()
        );
    }

    #[test]
    fn correct_empty_ocr_returns_none() {
        let idx = test_index();
        assert!(match_with_correction(&idx, archive::Category::Media, "", None).is_none());
    }

    #[test]
    fn correct_uses_injected_override() {
        let idx = test_index();

        assert!(match_with_correction(&idx, archive::Category::Digital, "文明", None).is_none());

        let corrected = match_with_correction(
            &idx,
            archive::Category::Digital,
            "文明",
            Some(&DEFAULT_CORRECTION_OVERRIDES),
        )
        .expect("覆盖项应指定对应档案");
        assert_eq!(
            corrected.item_ids,
            vec![archive::ArchiveId::new("nar_digital_map02_13003_1")]
        );
    }

    /// 全量验证：加载真实 prts.json，把每个标题模拟成「截断 / 替换」后的 OCR 输出，
    /// 检查纠错是否命中原条目（标题一致即可，同标题多条允许命中任一）。
    #[test]
    fn correct_all_real_titles() {
        let app_paths = AppPaths::new().unwrap();
        let idx = archive::Database::load(&app_paths).expect("加载真实档案目录失败");

        let mut total = 0usize;
        let mut hit = 0usize;
        let mut miss: Vec<(archive::ArchiveId, String, String)> = Vec::new();

        for item in &idx.catalog().archives {
            let id = &item.id;
            let title = &item.title;
            let category_id = item.category;
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
            match match_with_correction(
                &idx,
                category_id,
                &ocr,
                Some(&DEFAULT_CORRECTION_OVERRIDES),
            ) {
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
