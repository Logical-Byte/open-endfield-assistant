import type { PrtsData } from '@/types/prts';
import { getPrtsData } from '@/utils/tauri';
import { ref } from 'vue';

/** prts.json 完整数据（加载完成前为 null） */
export const prtsData = ref<PrtsData | null>(null);

let initialized = false;

export function initPrtsData() {
  if (!initialized) {
    initialized = true;

    getPrtsData().then((data) => {
      prtsData.value = data;
    });
  }
}
