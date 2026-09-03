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
/// 处理流程为：移除富文本标签，转换半角标点，截取前 [`NORM_MAX_CHARS`]
/// 个字符，移除空白与遮罩字符，并应用已知字符替换。`ArchiveTitleIndex`
/// 存储候选标题的规范化字符串，档案扫描使用同一函数规范化 OCR 文本。
pub(crate) fn normalize(text: &str) -> String {
    let no_tags = strip_rich_text_tags(text);
    let half_to_full: String = no_tags.chars().map(halfwidth_to_fullwidth).collect();
    let truncated: String = half_to_full.chars().take(NORM_MAX_CHARS).collect();
    truncated
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() || matches!(c, '\u{200b}' | '\u{3000}' | '■') {
                None
            } else {
                Some(match c {
                    '決' => '决',
                    _ => c,
                })
            }
        })
        .collect()
}

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
