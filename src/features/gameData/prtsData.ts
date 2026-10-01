import type { PrtsData } from '@/features/gameData/types/prts';
import { getPrtsData } from '@/features/gameData/ipc';
import { ref } from 'vue';

/** prts.json 完整数据（加载完成前为 null） */
export const prtsData = ref<PrtsData | null>(null);

export async function initPrtsData() {
  prtsData.value = await getPrtsData();
}
