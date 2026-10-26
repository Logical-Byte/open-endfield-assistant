// 开发服务器中的临时设计数据，独立于实际档案目录和扫描结果。
import { ref, shallowRef } from 'vue';
import { ArchiveCatalog } from '@/features/gameData/archiveCatalog';
import type {
  AcquisitionMethod,
  ArchiveEntry,
  ArchiveId,
  CategoryEntry,
} from '@/shared/types/archive';
import type { ArchiveScanSource } from './useArchiveScanResults';
import type { ScannedItemId, ScannedItemRecord } from './types/scannedItem';

export function createScanPreviewSource(): ArchiveScanSource {
  const categories: CategoryEntry[] = [
    { id: 'paper', page: 'text', name: { 'zh-CN': '纸质记录', 'en-US': 'Paper records' } },
    { id: 'digital', page: 'text', name: { 'zh-CN': '电子档案', 'en-US': 'Digital records' } },
    { id: 'report', page: 'document', name: { 'zh-CN': '调查报告', 'en-US': 'Reports' } },
  ];
  const titles = [
    '研究人员关于旧设施异常现象、长期观测结果与后续调查计划的补充笔记',
    '值班记录',
    '一封未寄出的信',
    '关于旧设施维护与人员交接的补充说明',
    '应急疏散方案',
  ];
  const methods: AcquisitionMethod[] = ['map', 'mission', 'auto', 'shop', 'invstgt'];
  const archives: ArchiveEntry[] = titles.map((title, index): ArchiveEntry => {
    return {
      id: `design-preview-${index}` as ArchiveId,
      category: categories[index % categories.length]!.id,
      title: { 'zh-CN': title, 'en-US': title },
      ...(index < 3 && {
        contentPreview: {
          'zh-CN':
            index === 0
              ? '示例正文：本记录汇总了连续多日的设施巡检结果、异常信号出现的时间与位置，以及研究人员对可能原因的初步判断。后续调查还需要结合现场截图、设备运行日志和人员交接记录，逐项确认相关线索，并补充尚未完成的观测项目。'
              : '示例正文：此记录仅用于观察排版、间距与按钮状态。',
          'en-US':
            index === 0
              ? 'This preview summarizes several days of facility inspections, the time and location of unusual signals, and preliminary observations. Further investigation will compare screenshots, equipment logs, and handover notes to verify each finding and complete the remaining observations.'
              : 'Design preview only.',
        },
      }),
      acquisitionMethod: methods[index % methods.length]!,
    };
  });
  const data = shallowRef(
    new ArchiveCatalog({
      pages: [
        { id: 'text', name: { 'zh-CN': '见闻辑录', 'en-US': 'Records' } },
        { id: 'document', name: { 'zh-CN': '中枢档案', 'en-US': 'Documents' } },
      ],
      categories,
      archives,
    }),
  );
  const scans = ref<ScannedItemRecord[]>(
    archives.map((archive, index): ScannedItemRecord => {
      const category = categories.find((entry): boolean => entry.id === archive.category)!;
      const canvas = document.createElement('canvas');
      canvas.width = 1280;
      canvas.height = 720;
      const context = canvas.getContext('2d')!;
      context.fillStyle = '#252529';
      context.fillRect(0, 0, 1280, 720);
      context.fillStyle = '#eeeeee';
      context.font = '26px sans-serif';
      context.fillText(archive.title['zh-CN'], 372, 100);
      context.font = '18px sans-serif';
      context.fillText('排版示例图，非真实游戏截图', 372, 180);
      return {
        scannedItemId: (index + 1) as ScannedItemId,
        gameLocale: 'zh-CN',
        foundInPage: category.page,
        foundInCategory: category.id,
        image: canvas.toDataURL('image/png'),
        ocrResult:
          index % 3 === 0 ? `${archive.title['zh-CN']}（识别有误）` : archive.title['zh-CN'],
        matching:
          index % 3 === 0
            ? {
                kind: 'recommendations',
                archiveIds: [archive.id, archives[(index + 3) % archives.length]!.id],
              }
            : { kind: 'exact', archiveId: archive.id },
        manualChoice:
          index % 5 === 0 ? { kind: 'manual', archiveId: archive.id } : { kind: 'automatic' },
      };
    }),
  );

  return {
    data,
    scans,
    choose(id: ScannedItemId, archiveId: ArchiveId | null): void {
      scans.value = scans.value.map((scan): ScannedItemRecord =>
        scan.scannedItemId === id ? { ...scan, manualChoice: { kind: 'manual', archiveId } } : scan,
      );
    },
    clear(): void {
      scans.value = [];
    },
    restore(previous: Readonly<ScannedItemRecord>): void {
      scans.value = scans.value.map((scan): ScannedItemRecord =>
        scan.scannedItemId === previous.scannedItemId
          ? { ...scan, manualChoice: previous.manualChoice }
          : scan,
      );
    },
  };
}
