//! 共享应用状态（模块级单例 ref，多个组件共享）。
import { getStatus, onAppStatus } from '@/utils/tauri';
import { ref } from 'vue';

/** 主任务是否正在运行 */
const appStatus = ref({ running: false });

let initialized = false;

export function useAppState() {
  if (!initialized) {
    initialized = true;

    getStatus().then((s) => {
      appStatus.value = s;
    });
    onAppStatus((s) => {
      appStatus.value = s;
    });
  }

  return {
    appStatus,
  };
}
