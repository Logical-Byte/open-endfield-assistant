//! 档案匹配：先判断标题，仅为已知同名组请求正文，再产出自动匹配建议。

use crate::{
    data::archive::{self, normalize},
    utils::region::{Region2D, ltwh},
};
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;

const SCORE_THRESHOLD: f64 = 0.80;

/// 后端提供的匹配建议，描述自动处理的判断，不包含人工选择。
///
/// 建议只有两种可能：
///
/// - 精确匹配（`Exact`）：在扫描所在分类内，规范化标题唯一命中一个档案 ID，
///   或由已知正文判别逻辑唯一确认同名组中的一个档案。
///   或直接标题查找没有命中，由已知标题解释规则精确命中指定 ID。
///   当前规则为电子档案的「文明」。该建议可自动建立单个档案的匹配。
/// - 无匹配，仅提供推荐（`Recommendations`）：规范化标题命中多个 ID 且正文未能确认，
///   或没有精确命中，只能通过模糊匹配寻找候选。空标题或没有合适候选也属于此分支。
///   推荐可以是零个、一个或多个，只有一个推荐也不能自动建立匹配。
///
/// 推荐只帮助用户选择，不构成已收集的证据。前端另行结合人工决定计算有效匹配，
/// 人工选择或清除不会改写这份后端建议。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) enum MatchResult {
    Exact {
        #[serde(rename = "archiveId")]
        archive_id: archive::ArchiveId,
    },
    Recommendations {
        #[serde(rename = "archiveIds")]
        archive_ids: Vec<archive::ArchiveId>,
    },
}

/// 一次标题判断的阶段性结果，决定扫描流程是否需要继续采集正文。
///
/// 大部分标题直接产生 `Complete`，包括没有专用正文判别逻辑的同名组。
/// `NeedsBody` 携带本次查询得到的候选和判别上下文，正文读取后无需再次查询目录。
/// 这是当前扫描项的内部状态，不是发送给前端的匹配建议。
#[derive(Debug)]
pub(super) enum TitleMatch {
    Complete(MatchResult),
    NeedsBody(BodyMatch),
}

/// 为一个已知同名组保留的正文判别上下文。
///
/// 候选沿用第一次标题查询的结果。消费上下文时，正文能够确认候选才自动匹配，
/// 否则原样返回这些推荐。该类型拥有候选 ID，不借用数据库，也不保存截图或 OCR 原文。
#[derive(Debug)]
pub(super) struct BodyMatch {
    candidates: Vec<archive::ArchiveId>,
    discriminator: BodyDiscriminator,
}

/// 已实际支持的同名组专用判别方式。
///
/// 当前只硬编码「挂在竹子上的字条」。它用首行前缀判断，但不要求将来其他同名组
/// 使用相同算法。各分支自行选择 OCR 区域与判断方式，无需建立通用正文规则表。
#[derive(Debug)]
enum BodyDiscriminator {
    BambooNotes,
}

impl BodyMatch {
    /// 返回该同名组已标定的正文区域，使用 1280 × 720 截图坐标。
    pub(super) fn ocr_roi(&self) -> Region2D<u32> {
        match self.discriminator {
            // 两份字条的首行布局一致，范围包含灰色遮罩，但避开标题和下一行正文。
            BodyDiscriminator::BambooNotes => ltwh!(380, 118, 545, 36),
        }
    }

    /// 使用正文完成本次判断。缺失、空白或无法确认的正文均保留原同名推荐。
    ///
    /// 只从第一次查询的候选中选取 ID，因此硬编码解释不会建立分类或同名组之外的匹配。
    /// 原始 OCR 观察由扫描记录保存，本函数仅生成用于比较的临时文本。
    pub(super) fn resolve(self, body: Option<&str>) -> MatchResult {
        let matched_id = match body {
            Some(body) => match self.discriminator {
                BodyDiscriminator::BambooNotes => match_bamboo_notes_body(body),
            },
            None => None,
        };

        if let Some(matched_id) = matched_id {
            for candidate in &self.candidates {
                if candidate.as_str() == matched_id {
                    return MatchResult::Exact {
                        archive_id: candidate.clone(),
                    };
                }
            }
        }

        MatchResult::Recommendations {
            archive_ids: self.candidates,
        }
    }
}

/// 「挂在竹子上的字条」的专用正文判断，不依赖资源中的 `contentPreview`。
///
/// 源正文与档案 ID 的关联已核实。第二份首行的「保佑寨子」之后是灰色遮罩，
/// 实测 OCR 只返回这四个字，因此不要求识别出源文本的 `■`，也不猜测被遮罩内容。
/// 两个 `starts_with` 允许后续多读到正文，但不容忍前缀错字或使用相似度自动确认。
fn match_bamboo_notes_body(body: &str) -> Option<&'static str> {
    // 正文不复用标题规范化：标题会截断字符，正文需保留跨行组合后的完整前缀。
    // 仅保留文字和数字，忽略排版空白、标点及遮罩符号。
    let body: String = body.chars().filter(|c| c.is_alphanumeric()).collect();
    if body.starts_with("先祖保佑我家那口子") {
        Some("nar_paper_map02_93_1")
    } else if body.starts_with("保佑寨子") {
        Some("nar_paper_map02_93_2")
    } else {
        None
    }
}

/// 在扫描分类内依次尝试规范化精确匹配、已知标题解释和模糊推荐。
/// 只有精确命中的已知同名组会返回 `NeedsBody`，模糊标题不会触发正文判断。
pub(super) fn match_title(
    index: &archive::Database,
    category: archive::Category,
    ocr_text: &str,
) -> TitleMatch {
    let normalized = normalize(ocr_text);
    if normalized.is_empty() {
        return TitleMatch::Complete(MatchResult::Recommendations {
            archive_ids: Vec::new(),
        });
    }
    if let Some(candidates) = index.by_normalized_title(category, &normalized) {
        let ids: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect();
        return if ids.len() == 1 {
            TitleMatch::Complete(MatchResult::Exact {
                archive_id: ids.into_iter().next().unwrap(),
            })
        } else if category == archive::Category::Paper && normalized == "挂在竹子上的字条" {
            TitleMatch::NeedsBody(BodyMatch {
                candidates: ids,
                discriminator: BodyDiscriminator::BambooNotes,
            })
        } else {
            TitleMatch::Complete(MatchResult::Recommendations { archive_ids: ids })
        };
    }
    // 「文明」是被遮罩标题的已知解释，精确命中指定档案。
    if category == archive::Category::Digital && normalized == "文明" {
        let id = archive::ArchiveId::new("nar_digital_map02_13003_1");
        if index.candidate_by_id(category, &id).is_some() {
            return TitleMatch::Complete(MatchResult::Exact { archive_id: id });
        }
    }
    let order: HashMap<_, _> = index
        .catalog()
        .archives
        .iter()
        .enumerate()
        .map(|(position, archive)| (&archive.id, position))
        .collect();
    let mut recommended = Vec::new();
    for (title, candidates) in index.normalized_groups(category) {
        let score = similarity(
            levenshtein(&normalized, title),
            normalized.chars().count(),
            title.chars().count(),
        );
        if score >= SCORE_THRESHOLD {
            for candidate in candidates.iter() {
                recommended.push((score, order[&candidate.id], candidate.id.clone()));
            }
        }
    }
    recommended.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.cmp(&right.1)));
    TitleMatch::Complete(MatchResult::Recommendations {
        archive_ids: recommended.into_iter().map(|(_, _, id)| id).collect(),
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
    use crate::app_paths::AppPaths;

    fn match_without_body(
        index: &archive::Database,
        category: archive::Category,
        title: &str,
    ) -> MatchResult {
        match match_title(index, category, title) {
            TitleMatch::Complete(result) => result,
            TitleMatch::NeedsBody(context) => context.resolve(None),
        }
    }

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
            (
                "nar_paper_map02_93_1",
                archive::Category::Paper,
                "挂在竹子上的字条",
            ),
            (
                "nar_paper_map02_93_2",
                archive::Category::Paper,
                "挂在竹子上的字条",
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
                    title: archive::LocalizedText {
                        zh_cn: title.to_string(),
                        en_us: title.to_string(),
                    },
                    content_preview: None,
                    acquisition_method: archive::AcquisitionMethod::Auto,
                })
                .collect(),
        })
    }

    #[test]
    fn known_duplicate_title_is_confirmed_by_body_without_resource_preview() {
        let index = test_index();
        for (body, id) in [
            ("先祖保佑我家那口子", "nar_paper_map02_93_1"),
            ("先祖，保佑我家\n那口子。平平安安去", "nar_paper_map02_93_1"),
            (" 保佑寨子■■■■ ", "nar_paper_map02_93_2"),
            ("保佑寨子，别出啥岔子", "nar_paper_map02_93_2"),
        ] {
            let TitleMatch::NeedsBody(context) =
                match_title(&index, archive::Category::Paper, "挂在竹子上的字条")
            else {
                panic!("已知同名组应请求正文识别")
            };
            assert_eq!(
                context.resolve(Some(body)),
                MatchResult::Exact {
                    archive_id: archive::ArchiveId::new(id),
                }
            );
        }
    }

    #[test]
    fn missing_partial_or_misread_body_preserves_original_duplicate_recommendations() {
        let index = test_index();
        for body in [
            None,
            Some(""),
            Some(" \n"),
            Some("保佑"),
            Some("先祖保佑我家那口了"),
            Some("别出啥岔子"),
        ] {
            let TitleMatch::NeedsBody(context) =
                match_title(&index, archive::Category::Paper, "挂在竹子上的字条")
            else {
                panic!("已知同名组应请求正文识别")
            };
            assert_eq!(
                context.resolve(body),
                MatchResult::Recommendations {
                    archive_ids: vec![
                        archive::ArchiveId::new("nar_paper_map02_93_1"),
                        archive::ArchiveId::new("nar_paper_map02_93_2")
                    ],
                }
            );
        }
    }

    #[test]
    fn normalized_unique_title_is_exact() {
        let index = test_index();
        assert_eq!(
            match_without_body(&index, archive::Category::Media, "決然工人的留声"),
            MatchResult::Exact {
                archive_id: archive::ArchiveId::new("nar_media_map01_108_1")
            }
        );
        assert_eq!(
            match_without_body(
                &index,
                archive::Category::Paper,
                "工团大会预算申报宣讲草稿（第八"
            ),
            MatchResult::Exact {
                archive_id: archive::ArchiveId::new("nar_paper_map01_122_1")
            }
        );
    }

    #[test]
    fn duplicate_exact_titles_only_recommend() {
        // 相同标题在另一个分类没有已知正文解释，直接保留推荐，不请求正文。
        let TitleMatch::Complete(result) = match_title(
            &test_index(),
            archive::Category::Digital,
            "挂在竹子上的字条",
        ) else {
            panic!("未支持的同名组不应请求正文")
        };
        assert_eq!(
            result,
            MatchResult::Recommendations {
                archive_ids: vec![
                    archive::ArchiveId::new("nar_dup_1"),
                    archive::ArchiveId::new("nar_dup_2")
                ],
            }
        );
    }

    #[test]
    fn single_fuzzy_candidate_does_not_match_automatically() {
        assert_eq!(
            match_without_body(&test_index(), archive::Category::Media, "决然工人的留生"),
            MatchResult::Recommendations {
                archive_ids: vec![archive::ArchiveId::new("nar_media_map01_108_1")]
            }
        );
    }

    #[test]
    fn fuzzy_recommends_all_qualifying_groups_without_score_gap() {
        assert_eq!(
            match_without_body(
                &test_index(),
                archive::Category::Paper,
                "天空观测记录（四号谷地"
            ),
            MatchResult::Recommendations {
                archive_ids: vec![
                    archive::ArchiveId::new("nar_paper_map01_59_1"),
                    archive::ArchiveId::new("nar_paper_map01_59_2")
                ]
            }
        );
    }

    #[test]
    fn fuzzy_expands_duplicate_group() {
        assert_eq!(
            match_without_body(
                &test_index(),
                archive::Category::Digital,
                "挂在竹子上的纸条"
            ),
            MatchResult::Recommendations {
                archive_ids: vec![
                    archive::ArchiveId::new("nar_dup_1"),
                    archive::ArchiveId::new("nar_dup_2")
                ]
            }
        );
    }

    #[test]
    fn empty_and_unrelated_ocr_have_no_recommendations() {
        for text in ["", "完全无关"] {
            assert_eq!(
                match_without_body(&test_index(), archive::Category::Media, text),
                MatchResult::Recommendations {
                    archive_ids: Vec::new()
                }
            );
        }
    }

    #[test]
    fn known_title_interpretation_is_exact_but_does_not_override_direct_hits() {
        let index = test_index();
        assert_eq!(
            match_without_body(&index, archive::Category::Digital, "文明"),
            MatchResult::Exact {
                archive_id: archive::ArchiveId::new("nar_digital_map02_13003_1")
            }
        );
        let mut catalog = index.catalog().clone();
        for id in ["direct_1", "direct_2"] {
            catalog.archives.push(archive::ArchiveEntry {
                id: archive::ArchiveId::new(id),
                category: archive::Category::Digital,
                title: archive::LocalizedText {
                    zh_cn: "文明".into(),
                    en_us: "Civilization".into(),
                },
                content_preview: None,
                acquisition_method: archive::AcquisitionMethod::Auto,
            });
        }
        assert_eq!(
            match_without_body(
                &archive::Database::from_catalog(catalog),
                archive::Category::Digital,
                "文明"
            ),
            MatchResult::Recommendations {
                archive_ids: vec![
                    archive::ArchiveId::new("direct_1"),
                    archive::ArchiveId::new("direct_2")
                ]
            }
        );
    }

    #[test]
    fn tied_fuzzy_candidates_follow_catalog_order() {
        let index = test_index();
        assert_eq!(
            match_without_body(&index, archive::Category::Paper, "天空观测记录（六号谷地）"),
            MatchResult::Recommendations {
                archive_ids: vec![
                    archive::ArchiveId::new("nar_paper_map01_59_1"),
                    archive::ArchiveId::new("nar_paper_map01_59_2")
                ]
            }
        );
    }

    #[test]
    fn real_titles_match_or_recommend_the_observed_archive() -> anyhow::Result<()> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let index = archive::Database::load(&AppPaths::with_root_dir(root))?;
        for archive in &index.catalog().archives {
            if normalize(&archive.title.zh_cn).is_empty() {
                continue;
            }
            let result = match_without_body(&index, archive.category, &archive.title.zh_cn);
            match result {
                MatchResult::Exact { archive_id } => assert_eq!(archive_id, archive.id),
                MatchResult::Recommendations { archive_ids } => {
                    assert!(archive_ids.contains(&archive.id), "候选缺少 {}", archive.id)
                }
            }
        }
        Ok(())
    }
}
