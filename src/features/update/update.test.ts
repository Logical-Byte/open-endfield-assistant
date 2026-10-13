import type { EventCallback } from '@tauri-apps/api/event';
import { nextTick } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { RunOutcome, Status } from '@/features/automation/types';
import type { Settings } from '@/features/settings/types/settings';
import type { UpdateStatus } from '@/features/update/types/update';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn<(command: string) => Promise<unknown>>(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke, Channel: class {} }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (event: string, callback: EventCallback<Status>): Promise<() => void> => {
    if (event === 'automation-status-changed') onAutomationStatus = callback;
    return () => {};
  },
}));

const updateInfo = { versionName: '1.3.0', releaseNote: 'notes' };
let backendStatus: UpdateStatus;
let saveResult: Promise<void>;
let onAutomationStatus: EventCallback<Status>;
let update: typeof import('./update');
let settings: typeof import('@/features/settings/settings');

async function settle(): Promise<void> {
  // 等待设置保存的 IPC Promise 链完成，再刷新响应式监听。
  await new Promise<void>((resolve) => setImmediate(resolve));
  await nextTick();
}

async function publishAutomationStatus(status: Status): Promise<void> {
  onAutomationStatus({ event: 'automation-status-changed', id: 1, payload: status });
  await settle();
}

beforeEach(async () => {
  vi.resetModules();
  invoke.mockReset();
  backendStatus = { operation: 'idle', availableUpdate: updateInfo, pendingUpdate: null };
  saveResult = Promise.resolve();
  invoke.mockImplementation(async (command) => {
    switch (command) {
      case 'load_oea_settings':
        return {
          uiLocale: 'en-US',
          majorVersion: 0,
          minorVersion: 0,
          minimizeToTray: false,
          soundVolume: 0.5,
          updateSource: 'mirrorchyan',
          mirrorchyanCdkEncrypted: '',
          updateProxyMode: 'system',
          updateProxyUrl: '',
          autoDownloadUpdates: false,
          autoInstallUpdates: true,
          scanTipsDismissedVersion: 0,
        } satisfies Settings;
      case 'save_oea_settings':
        return saveResult;
      case 'get_automation_status':
        return { state: 'idle', lastRun: null };
      case 'get_update_status':
        return backendStatus;
      case 'consume_startup_update_result':
        return 'no_transaction';
      case 'check_update':
        return { status: 'available', update: updateInfo };
      case 'download_update':
        backendStatus = { ...backendStatus, pendingUpdate: updateInfo };
        return { status: 'completed', update: updateInfo };
      case 'install_update':
        // 安装接管后应用会退出，保留在途请求以观察是否重复发起。
        return new Promise<void>(() => {});
      case 'log_debug':
      case 'log_warn':
      case 'log_error':
        return;
      default:
        throw new Error(`未预期的 Tauri 命令：${command}`);
    }
  });
  const automation = await import('@/features/automation/state');
  await automation.initAutomationState();
  settings = await import('@/features/settings/settings');
  await settings.initOeaSettings();
  update = await import('./update');
  await update.initUpdateState();
  await settle();
  invoke.mockClear();
});

describe('自动安装编排', () => {
  it('没有待安装更新时，扫描结束不发起安装也不写更新日志', async () => {
    for (let run = 0; run < 2; run++) {
      await publishAutomationStatus({ state: 'running', taskKind: 'archiveScan' });
      await publishAutomationStatus({
        state: 'idle',
        lastRun: { taskKind: 'archiveScan', outcome: { status: 'completed' } },
      });
    }
    expect(invoke).not.toHaveBeenCalled();
  });

  it('空闲时下载完成只发起一次安装', async () => {
    await update.startDownload();
    await settle();
    expect(invoke.mock.calls.filter(([command]) => command === 'install_update')).toHaveLength(1);
    await update.refreshUpdateStatus();
    await settle();
    expect(invoke.mock.calls.filter(([command]) => command === 'install_update')).toHaveLength(1);
  });

  it.each([
    { name: '完成', outcome: { status: 'completed' } },
    {
      name: '失败',
      outcome: { status: 'failed', error: { scope: 'worker', kind: 'navigationFailed' } },
    },
    {
      name: '停止',
      outcome: {
        status: 'failed',
        error: { scope: 'worker', kind: 'capability', reason: { kind: 'stoppedByUser' } },
      },
    },
  ] satisfies { name: string; outcome: RunOutcome }[])(
    '扫描$name后安装已下载更新，等待期间不写日志',
    async ({ outcome }) => {
      await publishAutomationStatus({ state: 'running', taskKind: 'archiveScan' });
      await update.startDownload();
      await settle();
      expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
      invoke.mockClear();
      await publishAutomationStatus({ state: 'stopping', taskKind: 'archiveScan' });
      expect(invoke).not.toHaveBeenCalled();
      await publishAutomationStatus({
        state: 'idle',
        lastRun: { taskKind: 'archiveScan', outcome },
      });
      expect(invoke.mock.calls.filter(([command]) => command === 'install_update')).toHaveLength(1);
    },
  );

  it('已有待安装更新时，开启自动安装须等设置保存成功', async () => {
    settings.editSettings({ autoInstallUpdates: false });
    await settle();
    await update.startDownload();
    await settle();
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);

    let completeSave!: () => void;
    saveResult = new Promise<void>((resolve) => {
      completeSave = resolve;
    });
    settings.editSettings({ autoInstallUpdates: true });
    await settle();
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    completeSave();
    await settle();
    expect(invoke.mock.calls.filter(([command]) => command === 'install_update')).toHaveLength(1);
  });
});
