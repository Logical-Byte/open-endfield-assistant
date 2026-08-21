import type { PrtsData } from '@/types/prts';
import { getPrtsData } from '@/utils/tauri';
import { ref } from 'vue';

/** prts.json 完整数据（加载完成前为 null） */
export const prtsData = ref<PrtsData | null>(null);

export async function initPrtsData() {
  prtsData.value = await getPrtsData();
}
