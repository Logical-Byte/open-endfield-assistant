use super::*;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use std::io::Write;

pub(super) struct Archive {
    pub id: catalog::ArchiveId,
    pub category_id: catalog::Category,
    pub category: String,
    pub title: String,
    pub acquisition: String,
    pub content: Option<String>,
    pub map: bool,
}
pub(super) fn project(catalog: &catalog::Catalog) -> Vec<Archive> {
    catalog
        .archives
        .iter()
        .map(|a| Archive {
            id: a.id.clone(),
            category_id: a.category,
            category: catalog
                .categories
                .iter()
                .find(|c| c.id == a.category)
                .map(|c| c.name.zh_cn.clone())
                .unwrap_or_default(),
            title: a.title.zh_cn.clone(),
            content: a.content_preview.as_ref().map(|c| c.zh_cn.clone()),
            map: a.acquisition_method == catalog::AcquisitionMethod::Map,
            acquisition: match a.acquisition_method {
                catalog::AcquisitionMethod::Map => "地图拾取",
                catalog::AcquisitionMethod::Mission => "任务获取",
                catalog::AcquisitionMethod::Auto => "自动解锁",
                catalog::AcquisitionMethod::Shop => "商店购买",
                catalog::AcquisitionMethod::Investigate => "研究提交",
            }
            .into(),
        })
        .collect()
}
pub(super) struct ScanRecord {
    pub id: u64,
    pub observation: archive_scan::ScannedItem,
    pub target: Option<usize>,
    /// `None` 是自动决定，`Some(None)` 是人工明确清除，不能退回自动采纳。
    pub manual_choice: Option<Option<catalog::ArchiveId>>,
    pub image: Option<Arc<gpui::Image>>,
}
impl ScanRecord {
    pub fn new(id: u64, observation: archive_scan::ScannedItem, archives: &[Archive]) -> Self {
        let image = decode_image(&observation.image);
        let mut record = Self {
            id,
            observation,
            target: None,
            manual_choice: None,
            image,
        };
        record.refresh(archives);
        record
    }
    pub fn refresh(&mut self, archives: &[Archive]) {
        let id = effective_id(&self.observation.matching, &self.manual_choice);
        self.target = id.and_then(|id| archives.iter().position(|a| &a.id == id));
    }
}
fn effective_id<'a>(
    matching: &'a archive_scan::MatchResult,
    manual: &'a Option<Option<catalog::ArchiveId>>,
) -> Option<&'a catalog::ArchiveId> {
    if let Some(choice) = manual {
        return choice.as_ref();
    }
    match matching {
        archive_scan::MatchResult::Exact { archive_id } => Some(archive_id),
        archive_scan::MatchResult::Recommendations { archive_ids } if archive_ids.len() == 1 => {
            archive_ids.first()
        }
        _ => None,
    }
}
pub(super) fn decode_image(data: &str) -> Option<Arc<gpui::Image>> {
    STANDARD
        .decode(data.strip_prefix("data:image/png;base64,")?)
        .ok()
        .map(|bytes| Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Png, bytes)))
}
pub(super) fn export_url(archives: &[Archive], records: &[ScanRecord]) -> anyhow::Result<String> {
    let collected: HashSet<_> = records.iter().filter_map(|r| r.target).collect();
    let (mut yes, mut no) = (vec![], vec![]);
    for (i, a) in archives.iter().enumerate() {
        if collected.contains(&i) {
            yes.push(a.id.as_str());
        } else {
            no.push(a.id.as_str());
        }
    }
    let version = env!("OEA_VERSION");
    let data = serde_json::json!({"majorVersion":0,"minorVersion":0,"data":{"oeaVersion":version,"prtsAllItems":{"collected":yes,"notCollected":no}}});
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&serde_json::to_vec(&data)?)?;
    Ok(format!(
        "https://oem.re/i/OEA-0-{}",
        URL_SAFE_NO_PAD.encode(gzip.finish()?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn manual_clear_and_undo_preserve_the_original_suggestion() {
        let id = catalog::ArchiveId::new("archive");
        let matching = archive_scan::MatchResult::Recommendations {
            archive_ids: vec![id.clone()],
        };
        assert_eq!(effective_id(&matching, &None), Some(&id));
        assert_eq!(effective_id(&matching, &Some(None)), None);
        let chosen = Some(Some(catalog::ArchiveId::new("chosen")));
        assert_eq!(effective_id(&matching, &chosen).unwrap().as_str(), "chosen");
        assert_eq!(effective_id(&matching, &None), Some(&id));
    }
    #[::core::prelude::v1::test]
    fn ambiguous_recommendations_do_not_collect_an_archive() {
        let matching = archive_scan::MatchResult::Recommendations {
            archive_ids: vec![catalog::ArchiveId::new("a"), catalog::ArchiveId::new("b")],
        };
        assert!(effective_id(&matching, &None).is_none());
    }
}
