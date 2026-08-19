import type { ArchiveAcquisitionMethod, ArchiveContract } from '@/types/archiveContract';
import { getArchiveContract } from '@/utils/tauri';
import { computed, ref } from 'vue';

/** 档案获取契约数据（加载完成前为 null） */
export const archiveContract = ref<ArchiveContract | null>(null);

/** 档案 id → 获取方式 查询表（契约数据加载后构建） */
const methodByArchiveId = computed(() => {
  const map = new Map<string, ArchiveAcquisitionMethod>();
  for (const rows of Object.values(archiveContract.value?.categories ?? {})) {
    for (const row of rows) {
      map.set(row.id, row.acquisition.method);
    }
  }
  return map;
});

/** 查询档案 id 对应的获取方式（未收录时返回 null）。 */
export function getAcquisitionMethod(archiveId: string): ArchiveAcquisitionMethod | null {
  return methodByArchiveId.value.get(archiveId) ?? null;
}

export async function initArchiveContract() {
  archiveContract.value = await getArchiveContract();
}
