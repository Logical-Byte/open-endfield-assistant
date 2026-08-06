//! 共享扫描结果状态（模块级单例 ref，多个组件共享）。
import type { ScanResult } from '@/types/scanResult';
import { onScanResult } from '@/utils/tauri';
import { ref } from 'vue';

/** 扫描结果列表（随扫描进度实时追加，按序号排序） */
const scanResults = ref<ScanResult[]>([]);

/** 模拟数据用到的档案库大类 id（pageType，循环使用） */
const MOCK_PAGE_TYPES = ['multi_media', 'text', 'document'] as const;

/** 模拟数据用到的档案库小类 id（categoryId，与 pageType 对应） */
const MOCK_SUB_CATEGORIES = ['media', 'paper', 'digital'] as const;

/** 假截图尺寸（与真实截图一致，便于测试裁剪区域） */
const MOCK_IMAGE_WIDTH = 1280;
const MOCK_IMAGE_HEIGHT = 720;

let initialized = false;

/**
 * 生成一张假的档案详情截图（1280x720 占位图，base64 PNG data URL）。
 * 内容为模拟的档案详情页，裁剪区域（OCR 文本区）绘制了占位文字。
 */
function createMockImage(index: number): string {
  const canvas = document.createElement('canvas');
  canvas.width = MOCK_IMAGE_WIDTH;
  canvas.height = MOCK_IMAGE_HEIGHT;
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    return '';
  }

  // 背景渐变
  const bg = ctx.createLinearGradient(0, 0, MOCK_IMAGE_WIDTH, MOCK_IMAGE_HEIGHT);
  bg.addColorStop(0, '#1e293b');
  bg.addColorStop(1, '#0f172a');
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, MOCK_IMAGE_WIDTH, MOCK_IMAGE_HEIGHT);

  // 顶栏（模拟页面标题区）
  ctx.fillStyle = '#334155';
  ctx.fillRect(0, 0, MOCK_IMAGE_WIDTH, 90);
  ctx.fillStyle = '#e2e8f0';
  ctx.font = 'bold 42px sans-serif';
  ctx.textBaseline = 'middle';
  ctx.fillText(`档案详情 #${index}`, 48, 45);

  // 裁剪区域（x: 343-936，y: 22-168）内绘制若干文本行，模拟 OCR 识别区
  ctx.font = '26px sans-serif';
  ctx.fillStyle = '#f8fafc';
  const lines = [
    `模拟档案 #${index}`,
    '类型：音像存档',
    '描述：这是一条用于前端测试的模拟 OCR 识别结果……',
  ];
  lines.forEach((line, i) => {
    ctx.fillText(line, 360, 60 + i * 36);
  });

  // 右下角水印，便于辨认是模拟数据
  ctx.fillStyle = '#475569';
  ctx.font = '20px sans-serif';
  ctx.fillText('mock-data', MOCK_IMAGE_WIDTH - 160, MOCK_IMAGE_HEIGHT - 40);

  return canvas.toDataURL('image/png');
}

/**
 * 生成一条假的扫描结果并追加到列表（仅前端，不经过后端），便于本地测试。
 * 类别按序号循环，每 3 条生成一条 failed、每 5 条生成一条 unrecognized 以覆盖不同状态。
 */
function pushMockScanResult(): void {
  const index = scanResults.value.reduce((max, r) => Math.max(max, r.index), 0) + 1;
  const category = MOCK_PAGE_TYPES[(index - 1) % MOCK_PAGE_TYPES.length];
  const subCategory = MOCK_SUB_CATEGORIES[(index - 1) % MOCK_SUB_CATEGORIES.length];
  const status: ScanResult['status'] =
    index % 3 === 0 ? 'failed' : index % 5 === 0 ? 'unrecognized' : 'success';
  const ocr = status === 'failed' ? '' : `模拟档案 #${index} 的 OCR 识别结果，用于前端测试。`;
  const corrected = status === 'success' ? `纠错后的档案标题 #${index}` : null;
  scanResults.value.push({
    status,
    index,
    category,
    subCategory,
    image: createMockImage(index),
    // 展示值优先取纠错结果（与后端事件处理保持一致）
    ocrResult: corrected ?? ocr,
    correctedTitle: corrected,
    itemIds: status === 'success' ? [`mock_item_${index}`] : [],
  });
  scanResults.value.sort((a, b) => a.index - b.index);
}

/** 清空扫描结果列表。 */
function clearScanResults(): void {
  scanResults.value = [];
}

export function useScanResults() {
  if (!initialized) {
    initialized = true;

    onScanResult((result) => {
      // 展示值优先取纠错后的标题（无法识别时保留 OCR 原文供用户手动编辑）
      scanResults.value.push({
        ...result,
        ocrResult: result.correctedTitle ?? result.ocrResult,
      });
      // 按序号排序，防止事件乱序导致展示错乱
      scanResults.value.sort((a, b) => a.index - b.index);
    });
  }

  return {
    scanResults,
    pushMockScanResult,
    clearScanResults,
  };
}
