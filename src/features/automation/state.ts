import type * as Automation from './types';
import { getAutomationStatus, onAutomationRunFinished, onAutomationStatus } from './ipc';
import {
  handleArchiveScanStatus,
  handleArchiveScanRunFinished,
} from '@/features/archiveScan/results';
import { ref } from 'vue';

/** 后端自动化运行状态的前端投影。 */
export const appStatus = ref<Automation.Status>({ state: 'idle' });

export async function initAppStatus(): Promise<void> {
  appStatus.value = await getAutomationStatus();
  await onAutomationStatus((status) => {
    appStatus.value = status;
    handleArchiveScanStatus(status);
  });
  await onAutomationRunFinished(handleArchiveScanRunFinished);
}
