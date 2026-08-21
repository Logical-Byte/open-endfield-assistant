//! 导出扫描结果到地图集（OEM）。
//!
//! 流程：构建 upload 数据 → gzip 压缩（单流、无文件名）→ URL-safe Base64 → 打开
//! `https://oem.re/i/<base64>`（由 opener 插件交给系统浏览器）。

import type { UploadData } from '@/types/upload';
import { prtsData } from '@/utils/app/prtsData';
import { scanResults } from '@/utils/app/scanResults';
import { logDebug, logError, logInfo } from '@/utils/tauri';
import { openUrl } from '@tauri-apps/plugin-opener';
import { gzipSync, strToU8 } from 'fflate';

/** 地图集导入地址前缀（`<base64>` 为 gzip 压缩后 JSON 的 URL-safe Base64）。 */
const OEM_IMPORT_URL_PREFIX = 'https://oem.re/i/';

/**
 * 构建上传数据：已收集 / 未收集档案 id 列表。
 *
 * - 已收集 = 全部成功扫描（含人工纠错）命中的档案 id；
 * - 重名档案：同名档案中只要有一个已收集，全部视为已收集；
 * - 未收集 = 所有档案去掉已收集。
 * 两个列表均按 allItems 的展示顺序排列。
 */
export function buildUploadData(): UploadData {
  const allItems = prtsData.value?.allItems ?? {};
  const allIds = Object.keys(allItems);

  const collected = new Set<string>();
  for (const result of scanResults.value) {
    if (result.status === 'success') {
      for (const id of result.itemIds) {
        collected.add(id);
      }
    }
  }

  // 重名档案：只要有一个已收集，所有同名档案都视为已收集
  const idsByTitle = new Map<string, string[]>();
  for (const item of Object.values(allItems)) {
    const ids = idsByTitle.get(item.title) ?? [];
    ids.push(item.id);
    idsByTitle.set(item.title, ids);
  }
  for (const ids of idsByTitle.values()) {
    if (ids.some((id) => collected.has(id))) {
      for (const id of ids) collected.add(id);
    }
  }

  const notCollected = allIds.filter((id) => !collected.has(id));
  return {
    majorVersion: 0,
    minorVersion: 0,
    data: {
      oeaVersion: __OEA_VERSION__,
      prtsAllItems: {
        collected: allIds.filter((id) => collected.has(id)),
        notCollected,
      },
    },
  };
}

/** Uint8Array → URL-safe Base64（RFC 4648 §5 base64url，无 `=` 填充）。 */
function bytesToBase64Url(bytes: Uint8Array): string {
  // 参考 MDN：`Array.from` 逐个映射避免 `...` 展开的栈溢出，无需分块
  const binString = Array.from(bytes, (byte) => String.fromCodePoint(byte)).join('');
  return btoa(binString).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** 导出扫描结果到地图集：在系统浏览器中打开导入链接。 */
export async function exportToOem(): Promise<void> {
  if (prtsData.value === null) {
    logError('导出到地图集失败：档案库数据尚未加载');
    return;
  }
  try {
    // 构建上传数据
    const uploadData = buildUploadData();
    // JSON 序列化
    const json = JSON.stringify(uploadData);
    // gzip 压缩
    const gzip = gzipSync(strToU8(json));
    // URL-safe Base64
    const base64Url = bytesToBase64Url(gzip);
    // 构造导入链接
    const url = `${OEM_IMPORT_URL_PREFIX}OEA-0-${base64Url}`;
    logDebug(`导出到地图集数据：${json}`);
    logDebug(`导出到地图集链接：${url}`);
    openUrl(url);
    logInfo(`导出到地图集成功：${json.length} 字节 → ${base64Url.length} 字节`);
  } catch (error) {
    logError(`导出到地图集失败: ${String(error)}`);
  }
}
