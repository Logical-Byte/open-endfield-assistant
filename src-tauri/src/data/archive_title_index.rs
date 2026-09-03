use std::collections::HashMap;

use crate::data::PrtsData;

/// 匹配时保留的标题前缀长度；对应 OCR 区域通常只能识别标题首行。
pub(crate) const NORM_MAX_CHARS: usize = 15;

#[derive(Debug)]
pub(crate) struct Candidate {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) norm: String,
}

/// 归档标题候选索引：`categoryId` 到候选列表的映射。
#[derive(Debug, Default)]
pub struct ArchiveTitleIndex {
    pub(crate) candidates: HashMap<String, Vec<Candidate>>,
}

impl ArchiveTitleIndex {
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

    pub fn len(&self) -> usize {
        self.candidates.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
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
