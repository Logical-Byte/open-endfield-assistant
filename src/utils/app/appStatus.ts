import { getStatus, onAppStatus } from '@/utils/tauri';
import { ref } from 'vue';

/** 扫描档案库任务是否正在运行 */
export const appStatus = ref({ running: false });

let initialized = false;

export function initAppStatus() {
  if (!initialized) {
    initialized = true;

    getStatus().then((s) => {
      appStatus.value = s;
    });
    onAppStatus((s) => {
      appStatus.value = s;
    });
  }
}
