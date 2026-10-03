import { ref } from 'vue';

export type EssenceScanWorkerType = 'production' | 'simulation';

/** 仅保留在本次应用会话中的 worker 选择。 */
export const essenceScanWorkerType = ref<EssenceScanWorkerType>('production');
