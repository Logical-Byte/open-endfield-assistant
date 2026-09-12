import { UpdateInstallStageEvent } from '@/types/update';
import { installError, installLocalUpdatePackage, updateOperationBusy } from '@/utils/app/update';
import { logInfo } from '@/utils/tauri';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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

/** 从本地选择 ZIP，并从“下载完成”状态进入生产安装路径。 */
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

  let unlisten: (() => void) | null = null;
  try {
    const packagePath = await invoke<string | null>('developer_choose_update_package');
    appendTrace(
      packagePath === null
        ? 'native picker or confirmation cancelled'
        : `native confirmation accepted: ${packagePath}`,
    );
    if (packagePath === null) {
      return;
    }
    unlisten = await listen<UpdateInstallStageEvent>('update-install-stage', (event) => {
      appendTrace(`Rust stage event: ${event.payload.stage}`);
    });

    // Picker 和事件订阅期间可能已有普通更新开始安装。此后到 `startInstall`
    // 设置内部互斥标记之前不再让出事件循环，避免覆盖正在安装的包状态。
    if (refuseActiveUpdate()) {
      return;
    }
    appendTrace('entering production installer with the selected local package');
    appendTrace(`invoking Rust install_update: ${packagePath}`);
    const result = await installLocalUpdatePackage(packagePath);
    if (result === 'failed') {
      appendTrace(`developer flow failed: ${installError.value ?? 'Rust installer failed'}`);
    } else if (result === 'skipped') {
      appendTrace('developer install did not start because an install condition blocked it');
    }
  } catch (error) {
    appendTrace(`developer flow failed: ${String(error)}`);
  } finally {
    unlisten?.();
    developerInstallBusy.value = false;
  }
}
