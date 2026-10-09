import type {
  Catalog,
  ArchiveId,
  ArchiveEntry,
  Category,
  CategoryEntry,
  Page,
  PageEntry,
} from '@/shared/types/archive';
import { getArchiveCatalog } from './ipc';
import { shallowRef } from 'vue';

/** 静态目录及其只读查询，索引只在加载时构建一次。 */
export class ArchiveCatalog {
  private readonly archivesById: Map<ArchiveId, ArchiveEntry>;
  private readonly categoriesById: Map<Category, CategoryEntry>;
  private readonly pagesById: Map<Page, PageEntry>;
  private readonly archivesByTitle: Map<Category, Map<string, ArchiveId[]>> = new Map();

  constructor(readonly catalog: Catalog) {
    this.archivesById = new Map(catalog.archives.map((row) => [row.id, row]));
    this.categoriesById = new Map(catalog.categories.map((row) => [row.id, row]));
    this.pagesById = new Map(catalog.pages.map((row) => [row.id, row]));
    for (const archive of catalog.archives) {
      const titles = this.archivesByTitle.get(archive.category) ?? new Map<string, ArchiveId[]>();
      const ids = titles.get(archive.title) ?? [];
      ids.push(archive.id);
      titles.set(archive.title, ids);
      this.archivesByTitle.set(archive.category, titles);
    }
  }

  archive(id: ArchiveId): ArchiveEntry | undefined {
    return this.archivesById.get(id);
  }

  category(id: Category): CategoryEntry | undefined {
    return this.categoriesById.get(id);
  }

  page(id: Page): PageEntry | undefined {
    return this.pagesById.get(id);
  }

  titles(category: Category): readonly string[] {
    return [...(this.archivesByTitle.get(category)?.keys() ?? [])];
  }

  idsByTitle(category: Category, title: string): readonly ArchiveId[] {
    return this.archivesByTitle.get(category)?.get(title) ?? [];
  }
}

/** 加载后共享静态目录，不为不可变数据建立 Vue 深层代理。 */
export const archiveCatalog = shallowRef<ArchiveCatalog | null>(null);

export async function initArchiveCatalog(): Promise<void> {
  archiveCatalog.value = new ArchiveCatalog(await getArchiveCatalog());
}
