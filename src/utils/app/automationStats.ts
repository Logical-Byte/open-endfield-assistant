import type { CaptureSummary } from '@/types/automationStats';
import { onAppStatus, onAutomationRunFinished } from '@/utils/tauri';
import { ref } from 'vue';

/** 当前前端进程中最近一次已完成的自动化统计。 */
export const latestAutomationCapture = ref<CaptureSummary | null>(null);

export async function initAutomationStats(): Promise<void> {
  await onAutomationRunFinished((event) => {
    latestAutomationCapture.value = event.capture;
  });
  await onAppStatus((status) => {
    if (status.running) {
      latestAutomationCapture.value = null;
    }
  });
}
