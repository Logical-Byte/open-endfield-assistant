use super::{
    ArchiveEntry, ArchiveId, Catalog, Category, CategoryEntry, LocalizedText, Page, PageEntry,
    source, title_index,
};
use crate::{app_paths::AppPaths, data::load_resource_json};
use anyhow::{Context, ensure};
use std::collections::{HashMap, HashSet};

/// 只读档案数据，拥有一份目录和派生索引。加载成功后不再修改。
pub struct Database {
    catalog: Catalog,
    by_id: HashMap<ArchiveId, usize>,
    categories: HashMap<Category, usize>,
    pages: HashMap<Page, usize>,
    titles: title_index::ArchiveTitleIndex,
}

impl Database {
    /// 从上游资源加载，投影或关联错误使加载失败。
    pub fn load(app_paths: &AppPaths) -> anyhow::Result<Self> {
        let prts = load_resource_json::<source::Prts>(app_paths, "data/prts.json")?;
        let contract =
            load_resource_json::<source::Contract>(app_paths, "data/archive_contract.json")?;
        let catalog = project(prts, contract)
            .context("合并 data/prts.json 与 data/archive_contract.json 失败")?;
        Ok(Self::from_catalog(catalog))
    }

    pub(crate) fn from_catalog(catalog: Catalog) -> Self {
        let by_id = catalog
            .archives
            .iter()
            .enumerate()
            .map(|(i, row)| (row.id.clone(), i))
            .collect();
        let categories = catalog
            .categories
            .iter()
            .enumerate()
            .map(|(i, row)| (row.id, i))
            .collect();
        let pages = catalog
            .pages
            .iter()
            .enumerate()
            .map(|(i, row)| (row.id, i))
            .collect();
        let titles = title_index::ArchiveTitleIndex::new(&catalog.archives);
        Self {
            catalog,
            by_id,
            categories,
            pages,
            titles,
        }
    }

    /// 有序目录的只读视图，可直接作为前端传输契约。
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn archive(&self, id: &ArchiveId) -> Option<&ArchiveEntry> {
        self.by_id.get(id).map(|&i| &self.catalog.archives[i])
    }

    pub fn category(&self, id: Category) -> Option<&CategoryEntry> {
        self.categories
            .get(&id)
            .map(|&i| &self.catalog.categories[i])
    }

    pub fn page(&self, id: Page) -> Option<&PageEntry> {
        self.pages.get(&id).map(|&i| &self.catalog.pages[i])
    }

    /// 查询已有规范化标题对应的候选组。
    pub(crate) fn by_normalized_title(
        &self,
        category: Category,
        title: &str,
    ) -> Option<Candidates<'_>> {
        self.titles
            .by_normalized_title(category, title)
            .map(|indices| Candidates {
                archives: &self.catalog.archives,
                indices,
            })
    }

    /// 遍历分类内的规范化标题及候选组。
    pub(crate) fn normalized_groups(
        &self,
        category: Category,
    ) -> impl Iterator<Item = (&str, Candidates<'_>)> {
        self.titles
            .normalized_groups(category)
            .map(|(title, indices)| {
                (
                    title,
                    Candidates {
                        archives: &self.catalog.archives,
                        indices,
                    },
                )
            })
    }

    pub(crate) fn candidate_by_id(
        &self,
        category: Category,
        id: &ArchiveId,
    ) -> Option<&ArchiveEntry> {
        self.archive(id).filter(|entry| entry.category == category)
    }
}

fn project(prts: source::Prts, contract: source::Contract) -> anyhow::Result<Catalog> {
    ensure!(
        contract.version == 1,
        "不支持的获取契约版本: {}",
        contract.version
    );
    for (id, page) in &prts.pages {
        validate_text(&page.name, &format!("page {id:?}"), "name")?;
    }
    for (id, category) in &prts.categories {
        validate_text(&category.name, &format!("category {id:?}"), "name")?;
    }
    for (id, first_lv) in &prts.first_lv {
        validate_text(&first_lv.name, &format!("firstLv {id:?}"), "name")?;
    }
    for (id, archive) in &prts.archives {
        validate_text(&archive.name, &format!("archive {id}"), "name")?;
        validate_text(&archive.title, &format!("archive {id}"), "title")?;
    }
    let mut methods = HashMap::new();
    for (category, rows) in contract.categories {
        for row in rows {
            let archive = prts
                .archives
                .get(&row.id)
                .with_context(|| format!("获取契约含额外档案: {}", row.id))?;
            ensure!(
                archive.category_id == category,
                "获取契约分类不一致: {}",
                row.id
            );
            ensure!(
                methods
                    .insert(row.id.clone(), row.acquisition.method)
                    .is_none(),
                "获取契约档案 ID 重复: {}",
                row.id
            );
        }
    }

    let mut catalog = Catalog {
        pages: Vec::new(),
        categories: Vec::new(),
        archives: Vec::new(),
    };
    let mut seen_categories = HashSet::new();
    let mut seen_first_lv = HashSet::new();
    let mut seen_archives = HashSet::new();
    // 保留生成器的页面顺序，逐层消费关系，最终顺序与现有目录一致。
    for (page_id, page) in &prts.pages {
        ensure!(
            *page_id == page.page_type,
            "页面键与身份不一致: {page_id:?}"
        );
        catalog.pages.push(PageEntry {
            id: *page_id,
            name: page.name.clone(),
        });
        let mut categories = Vec::new();
        for id in &page.category_ids {
            let category = prts
                .categories
                .get(id)
                .with_context(|| format!("页面 {page_id:?} 引用缺失分类: {id:?}"))?;
            ensure!(
                category.category_id == *id && category.r#type == *page_id,
                "分类身份或页面归属不一致: {id:?}"
            );
            ensure!(seen_categories.insert(*id), "分类被重复引用: {id:?}");
            categories.push(category);
        }
        categories.sort_by_key(|category| category.order);
        for category in categories {
            catalog.categories.push(CategoryEntry {
                id: category.category_id,
                page: *page_id,
                name: category.name.clone(),
            });
            let mut first_levels = Vec::new();
            for id in &category.first_lv_ids {
                let first_lv = prts.first_lv.get(id).with_context(|| {
                    format!("分类 {:?} 引用缺失一级条目: {id:?}", category.category_id)
                })?;
                ensure!(
                    first_lv.first_lv_id == *id
                        && first_lv.category_id == category.category_id
                        && first_lv.r#type == *page_id,
                    "一级条目身份或归属不一致: {id:?}"
                );
                ensure!(
                    seen_first_lv.insert(id.clone()),
                    "一级条目被重复引用: {id:?}"
                );
                first_levels.push(first_lv);
            }
            first_levels.sort_by_key(|first_lv| first_lv.order);
            for first_lv in first_levels {
                let mut archives = Vec::new();
                for id in &first_lv.item_ids {
                    let archive = prts.archives.get(id).with_context(|| {
                        format!("一级条目 {:?} 引用缺失档案: {id}", first_lv.first_lv_id)
                    })?;
                    ensure!(
                        archive.id == *id
                            && archive.first_lv_id == first_lv.first_lv_id
                            && archive.category_id == category.category_id
                            && archive.r#type == *page_id,
                        "档案身份或归属不一致: {id}"
                    );
                    ensure!(seen_archives.insert(id.clone()), "档案被重复引用: {id}");
                    archives.push(archive);
                }
                archives.sort_by_key(|archive| archive.order);
                for archive in archives {
                    let method = methods
                        .remove(&archive.id)
                        .with_context(|| format!("档案缺少获取契约: {}", archive.id))?;
                    catalog.archives.push(ArchiveEntry {
                        id: archive.id.clone(),
                        category: category.category_id,
                        title: archive.title.clone(),
                        acquisition_method: method,
                    });
                }
            }
        }
    }
    for id in prts.categories.keys() {
        ensure!(seen_categories.contains(id), "分类未被页面引用: {id:?}");
    }
    for id in prts.first_lv.keys() {
        ensure!(seen_first_lv.contains(id), "一级条目未被分类引用: {id:?}");
    }
    for id in prts.archives.keys() {
        ensure!(seen_archives.contains(id), "档案未被一级条目引用: {id}");
    }
    Ok(catalog)
}

fn validate_text(text: &LocalizedText, entity: &str, field: &str) -> anyhow::Result<()> {
    for (locale, value) in [("zh-CN", &text.zh_cn), ("en-US", &text.en_us)] {
        ensure!(
            !value.trim().is_empty(),
            "{locale} {entity}.{field} 译文为空白"
        );
    }
    Ok(())
}

/// 候选的只读视图，目录位置和索引组织方式不向纠错模块暴露。
#[derive(Clone, Copy)]
pub(crate) struct Candidates<'a> {
    archives: &'a [ArchiveEntry],
    indices: &'a [usize],
}

impl<'a> Candidates<'a> {
    pub(crate) fn iter(self) -> impl Iterator<Item = &'a ArchiveEntry> {
        self.indices.iter().map(move |&index| &self.archives[index])
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::{app_paths::AppPaths, data::archive::normalize};
    use std::path::Path;

    #[test]
    fn real_resources_form_a_consistent_database() -> anyhow::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        // 复用生产加载路径，资源更新引入的投影与关系错误应直接使测试失败。
        let db = Database::load(&AppPaths::with_root_dir(root))?;
        let catalog = db.catalog();

        for page in &catalog.pages {
            assert!(std::ptr::eq(db.page(page.id).unwrap(), page));
        }
        for category in &catalog.categories {
            assert!(std::ptr::eq(db.category(category.id).unwrap(), category));
            assert!(db.page(category.page).is_some());
        }
        for archive in &catalog.archives {
            assert!(std::ptr::eq(db.archive(&archive.id).unwrap(), archive));
            assert!(db.category(archive.category).is_some());
            let title = normalize(&archive.title.zh_cn);
            let candidates = db
                .by_normalized_title(archive.category, &title)
                .unwrap_or_else(|| panic!("档案缺少标题候选组: {}", archive.id));
            assert!(
                candidates.iter().any(|row| std::ptr::eq(row, archive)),
                "标题候选组缺少档案: {}",
                archive.id
            );
        }
        Ok(())
    }
}
