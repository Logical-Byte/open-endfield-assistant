import type * as Automation from '@/types/automation';
import { getAutomationStatus, onAutomationRunFinished, onAutomationStatus } from '@/utils/tauri';
import { ref } from 'vue';

/** 后端自动化运行状态的前端投影。 */
export const appStatus = ref<Automation.Status>({ state: 'idle' });

/** 最近一次档案扫描失败的原因；后端只发送一次，不保存终态。 */
export const scanError = ref<string | null>(null);

export async function initAppStatus() {
  appStatus.value = await getAutomationStatus();
  await onAutomationStatus((status) => {
    appStatus.value = status;
    if (status.state === 'running' && status.taskKind === 'archiveScan') {
      scanError.value = null;
    }
  });
  await onAutomationRunFinished((finished) => {
    if (finished.taskKind === 'archiveScan' && finished.outcome.status === 'failed') {
      scanError.value = finished.outcome.error;
    }
  });
}
