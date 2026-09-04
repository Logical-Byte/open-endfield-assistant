//! 档案标题候选的派生索引。
//!
//! 本模块的 interface 只提供业务无关的只读查询，隐藏内部的 `HashMap` 组织方式。
//! 纠错、评分和候选决策等下游逻辑不属于该 interface。

use std::collections::HashMap;

use crate::data::PrtsData;

/// 匹配时保留的标题前缀长度；对应 OCR 区域通常只能识别标题首行。
pub(crate) const NORM_MAX_CHARS: usize = 15;

#[derive(Debug)]
pub(crate) struct Candidate {
    id: String,
    title: String,
}

impl Candidate {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }
}

/// 归档标题候选索引：按 `categoryId` 和规范化标题组织候选。
#[derive(Debug, Default)]
pub struct ArchiveTitleIndex {
    by_category: HashMap<String, HashMap<String, Vec<Candidate>>>,
}

impl ArchiveTitleIndex {
    /// 从 `prts.json` 构建候选索引。
    pub fn from_prts(prts: &PrtsData) -> Self {
        let mut by_category: HashMap<String, HashMap<String, Vec<Candidate>>> = HashMap::new();
        for item in prts.all_items.values() {
            by_category
                .entry(item.category_id.clone())
                .or_default()
                .entry(normalize(&item.title))
                .or_default()
                .push(Candidate {
                    id: item.id.clone(),
                    title: item.title.clone(),
                });
        }

        Self { by_category }
    }

    /// 查询分类中具有指定规范化标题的候选。
    ///
    /// `normalized_title` 必须是已经过 [`normalize`] 处理的字符串。
    pub(crate) fn by_normalized_title(
        &self,
        category_id: &str,
        normalized_title: &str,
    ) -> Option<&[Candidate]> {
        self.by_category
            .get(category_id)?
            .get(normalized_title)
            .map(Vec::as_slice)
    }

    /// 遍历分类中的规范化标题及其候选。
    pub(crate) fn normalized_groups(
        &self,
        category_id: &str,
    ) -> impl Iterator<Item = (&str, &[Candidate])> {
        self.by_category
            .get(category_id)
            .into_iter()
            .flat_map(|category| category.iter())
            .map(|(normalized_title, candidates)| {
                (normalized_title.as_str(), candidates.as_slice())
            })
    }

    /// 按档案条目 ID 查询分类中的候选。
    pub(crate) fn candidate_by_id(&self, category_id: &str, item_id: &str) -> Option<&Candidate> {
        self.normalized_groups(category_id)
            .flat_map(|(_, candidates)| candidates)
            .find(|candidate| candidate.id == item_id)
    }

    pub fn len(&self) -> usize {
        self.by_category
            .values()
            .flat_map(|category| category.values())
            .map(Vec::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_category.is_empty()
    }
}

/// 生成档案标题候选与 OCR 文本共用的规范化形式。
///
/// 处理流程为：移除富文本标签，截取前 [`NORM_MAX_CHARS`] 个字符，转换半角标点，
/// 移除空白与遮罩字符，并应用已知字符替换。`ArchiveTitleIndex`
/// 存储候选标题的规范化字符串，档案扫描使用同一函数规范化 OCR 文本。
pub(crate) fn normalize(text: &str) -> String {
    strip_rich_text_tags(text.chars())
        .take(NORM_MAX_CHARS)
        .map(halfwidth_to_fullwidth)
        .filter(|&c| !is_ignored(c))
        .map(replace_known_character)
        .collect()
}

fn is_ignored(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{200b}' | '\u{3000}' | '■')
}

fn replace_known_character(c: char) -> char {
    match c {
        '決' => '决',
        _ => c,
    }
}

fn strip_rich_text_tags<I>(input: I) -> impl Iterator<Item = char>
where
    I: Iterator<Item = char>,
{
    let mut input = input;
    let mut in_tag = false;
    std::iter::from_fn(move || {
        loop {
            let c = input.next()?;
            if c == '<' || c == '＜' {
                in_tag = true;
                continue;
            }
            if c == '>' || c == '＞' {
                in_tag = false;
                continue;
            }
            if !in_tag {
                return Some(c);
            }
        }
    })
}

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

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::{ArchiveTitleIndex, normalize};
    use crate::data::schema::{PrtsAllItem, PrtsData, PrtsPageType};

    fn prts_with_items(items: [(&str, &str, &str); 3]) -> PrtsData {
        let all_items = items
            .into_iter()
            .enumerate()
            .map(|(order, (id, category_id, title))| {
                (
                    id.to_string(),
                    PrtsAllItem {
                        category_id: category_id.to_string(),
                        first_lv_id: format!("{category_id}_1"),
                        id: id.to_string(),
                        name: title.to_string(),
                        order: order as i64,
                        title: title.to_string(),
                        r#type: PrtsPageType::Text,
                    },
                )
            })
            .collect();

        PrtsData {
            prts_page: IndexMap::new(),
            prts_category: IndexMap::new(),
            first_lv: IndexMap::new(),
            all_items,
        }
    }

    #[test]
    fn indexes_candidates_by_category_and_normalized_title() {
        let prts = prts_with_items([
            ("paper-1", "paper", "标题(A)"),
            ("paper-2", "paper", "标题（A）"),
            ("digital-1", "digital", "标题(A)"),
        ]);

        let index = ArchiveTitleIndex::from_prts(&prts);
        let paper = index
            .by_normalized_title("paper", &normalize("标题(A)"))
            .expect("paper 分类应包含规范化标题组");

        assert_eq!(
            paper
                .iter()
                .map(|candidate| candidate.id())
                .collect::<Vec<_>>(),
            vec!["paper-1", "paper-2"]
        );
        assert_eq!(index.normalized_groups("paper").count(), 1);
        assert_eq!(index.normalized_groups("digital").count(), 1);
        assert_eq!(
            index
                .candidate_by_id("digital", "digital-1")
                .map(|candidate| candidate.title()),
            Some("标题(A)")
        );
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn normalize_replaces_traditional_chars() {
        assert_eq!(normalize("決然工人的留声"), "决然工人的留声");
    }

    #[test]
    fn normalize_converts_halfwidth_parens() {
        assert_eq!(normalize("（第八版）"), "（第八版）");
        assert_eq!(normalize("(A)"), "（A）");
    }

    #[test]
    fn normalize_strips_rich_text_tags_and_blocks() {
        // 富文本标签 / ■ / 零宽空格 / 全角空格都应被剥离
        assert_eq!(normalize("<@nar.mark>\u{200b}■■</>文明"), "文明");
        assert_eq!(normalize("＜@nar.mark＞文明＜/＞"), "文明");
        assert_eq!(normalize("<@x>(A)</>"), "（A）");
        assert_eq!(normalize("A\u{3000}B"), "AB");
    }

    #[test]
    fn normalize_removes_all_ignored_characters() {
        assert_eq!(normalize("<tag> \u{200b}■■</tag>"), "");
    }

    #[test]
    fn normalize_truncates_to_15_chars() {
        assert_eq!(
            normalize("一二三四五六七八九十一二三四五六"),
            "一二三四五六七八九十一二三四五"
        );
    }

    #[test]
    fn normalize_preserves_ascii_letters_and_digits() {
        assert_eq!(normalize("ABC123"), "ABC123");
    }
}
