import type { AppStatus } from '@/types/appStatus';
import { getStatus, onAppStatus } from '@/utils/tauri';
import { ref } from 'vue';

/** 应用状态（运行标志 + 最近一次扫描任务的失败原因） */
export const appStatus = ref<AppStatus>({ running: false, scanError: null });

export async function initAppStatus() {
  appStatus.value = await getStatus();
  onAppStatus((s) => {
    appStatus.value = s;
  });
}
