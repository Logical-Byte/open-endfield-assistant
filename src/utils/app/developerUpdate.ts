import { installError, startDeveloperInstall, updateOperationBusy } from '@/utils/app/update';
import { logInfo } from '@/utils/tauri';
import { computed, ref } from 'vue';

/** 开发者选项手动安装流程的可见诊断日志。 */
export const developerInstallTrace = ref<string[]>([]);
/** 是否正在等待本地包选择或执行手动安装。 */
export const developerInstallBusy = ref<boolean>(false);

/** 开发者安装入口是否暂时不可用。 */
export const developerInstallUnavailable = computed<boolean>(
  () => developerInstallBusy.value || updateOperationBusy.value,
);

function appendTrace(message: string): void {
  const line = `${new Date().toLocaleTimeString()} ${message}`;
  developerInstallTrace.value.push(line);
  console.info(`[developer update] ${message}`);
  void logInfo(`[developer update] ${message}`).catch((error) => {
    developerInstallTrace.value.push(
      `${new Date().toLocaleTimeString()} Rust 日志转发失败: ${String(error)}`,
    );
  });
}

/** 更新流程正在使用共享状态时，拒绝覆盖其更新包。 */
function refuseActiveUpdate(): boolean {
  if (!updateOperationBusy.value) {
    return false;
  }
  appendTrace('another update operation owns the package state; refusing developer install');
  return true;
}

/** 在 Rust 内选择、暂存本地 ZIP 并进入生产安装路径。 */
export async function developerInstallUpdatePackage(): Promise<void> {
  if (developerInstallBusy.value) {
    return;
  }
  developerInstallTrace.value = [];
  appendTrace('developer button clicked');
  if (refuseActiveUpdate()) {
    return;
  }
  developerInstallBusy.value = true;
  appendTrace('invoking native Rust picker and confirmation');

  try {
    const result = await startDeveloperInstall((stage) => {
      appendTrace(`Rust stage event: ${stage}`);
    });
    if (result === 'failed') {
      appendTrace(`developer flow failed: ${installError.value ?? 'Rust installer failed'}`);
    } else if (result === 'cancelled') {
      appendTrace('native picker or confirmation cancelled');
    }
  } catch (error) {
    appendTrace(`developer flow failed: ${String(error)}`);
  } finally {
    developerInstallBusy.value = false;
  }
}
