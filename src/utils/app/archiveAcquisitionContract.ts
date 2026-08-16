import type {
  AcquisitionMethod,
  ArchiveAcquisitionContract,
} from '@/types/archiveAcquisitionContract';
import { getArchiveAcquisitionContract } from '@/utils/tauri';
import { computed, ref } from 'vue';

/** 档案获取契约数据（加载完成前为 null） */
export const acquisitionContracts = ref<ArchiveAcquisitionContract[] | null>(null);

/** 档案 id → 获取方式 查询表（契约数据加载后构建） */
const methodByArchiveId = computed(() => {
  const map = new Map<string, AcquisitionMethod>();
  for (const contract of acquisitionContracts.value ?? []) {
    map.set(contract.type, contract.method);
  }
  return map;
});

/** 查询档案 id 对应的获取方式（未收录时返回 null）。 */
export function getAcquisitionMethod(archiveId: string): AcquisitionMethod | null {
  return methodByArchiveId.value.get(archiveId) ?? null;
}

export async function initArchiveAcquisitionContract() {
  acquisitionContracts.value = await getArchiveAcquisitionContract();
}
