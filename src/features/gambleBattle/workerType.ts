import type { WorkerType } from '@/shared/types/automation';
import { computed, ref } from 'vue';

export type GambleBattleWorkerType = WorkerType;

/** 仅保留在本次应用会话中的 worker 选择。 */
export const gambleBattleWorkerType = ref<GambleBattleWorkerType>('production');

export const simulateGambleBattle = computed<boolean>({
  get: () => gambleBattleWorkerType.value === 'simulation',
  set: (enabled: boolean) => {
    gambleBattleWorkerType.value = enabled ? 'simulation' : 'production';
  },
});
