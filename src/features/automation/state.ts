import type * as Automation from './types';
import { getAutomationStatus, onAutomationStatusChanged } from './ipc';
import { readonly, shallowRef, type DeepReadonly, type Ref } from 'vue';

/** 后端自动化运行状态的前端投影。 */
const status = shallowRef<Automation.Status>({ state: 'idle', lastRun: null });
export const automationStatus: Readonly<Ref<DeepReadonly<Automation.Status>>> = readonly(status);

export async function initAutomationState(): Promise<void> {
  status.value = await getAutomationStatus();
  await onAutomationStatusChanged((nextStatus) => {
    status.value = nextStatus;
  });
}
