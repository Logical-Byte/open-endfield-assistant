//! OCR 候选索引，目录身份与标题不在索引中重复存储。

use super::{ArchiveEntry, Category};
use std::collections::HashMap;

/// OCR 标题区域通常只能识别标题首行。
pub(crate) const NORM_MAX_CHARS: usize = 15;

#[derive(Default)]
pub(super) struct ArchiveTitleIndex {
    by_category: HashMap<Category, HashMap<String, Vec<usize>>>,
}

impl ArchiveTitleIndex {
    pub(super) fn new(archives: &[ArchiveEntry]) -> Self {
        let mut by_category: HashMap<Category, HashMap<String, Vec<usize>>> = HashMap::new();
        for (index, entry) in archives.iter().enumerate() {
            by_category
                .entry(entry.category)
                .or_default()
                .entry(normalize(&entry.title))
                .or_default()
                .push(index);
        }
        Self { by_category }
    }

    pub(super) fn by_normalized_title(&self, category: Category, title: &str) -> Option<&[usize]> {
        self.by_category
            .get(&category)?
            .get(title)
            .map(Vec::as_slice)
    }

    pub(super) fn normalized_groups(
        &self,
        category: Category,
    ) -> impl Iterator<Item = (&str, &[usize])> {
        self.by_category
            .get(&category)
            .into_iter()
            .flat_map(|groups| groups.iter())
            .map(|(title, candidates)| (title.as_str(), candidates.as_slice()))
    }
}
/// 生成档案标题候选与 OCR 文本共用的规范化形式。
///
/// 处理流程为：移除富文本标签，截取前 [`NORM_MAX_CHARS`] 个字符，转换半角标点，
/// 移除空白与遮罩字符，并应用已知字符替换。索引构建与档案扫描共用此规则。
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
    use super::normalize;
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
