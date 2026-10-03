import type {
  ArchiveAcquisitionMethod,
  ArchiveContract,
} from '@/features/gameData/types/archiveContract';
import { getArchiveContract } from '@/features/gameData/ipc';
import { computed, ref, type ComputedRef, type Ref } from 'vue';

/** 档案获取契约数据（加载完成前为 null） */
export const archiveContract: Ref<ArchiveContract | null> = ref(null);

/** 档案 id → 获取方式 查询表（契约数据加载后构建） */
export const methodByArchiveId: ComputedRef<ReadonlyMap<string, ArchiveAcquisitionMethod>> =
  computed((): ReadonlyMap<string, ArchiveAcquisitionMethod> => {
    const methods: Map<string, ArchiveAcquisitionMethod> = new Map();
    for (const rows of Object.values(archiveContract.value?.categories ?? {})) {
      for (const row of rows) methods.set(row.id, row.acquisition.method);
    }
    return methods;
  });

/** 查询档案 id 对应的获取方式（未收录时返回 null）。 */
export function getAcquisitionMethod(archiveId: string): ArchiveAcquisitionMethod | null {
  return methodByArchiveId.value.get(archiveId) ?? null;
}

export async function initArchiveContract(): Promise<void> {
  archiveContract.value = await getArchiveContract();
}
