import { computed, ref } from 'vue';

export type ArchiveScanWorkerType = 'production' | 'simulation';

/** 仅保留在本次应用会话中的 worker 选择。 */
export const archiveScanWorkerType = ref<ArchiveScanWorkerType>('production');

export const simulateArchiveScan = computed<boolean>({
  get: () => archiveScanWorkerType.value === 'simulation',
  set: (enabled: boolean) => {
    archiveScanWorkerType.value = enabled ? 'simulation' : 'production';
  },
});
