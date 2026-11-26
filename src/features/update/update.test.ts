import type { EventCallback } from '@tauri-apps/api/event';
import { nextTick } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { RunOutcome, Status } from '@/features/automation/types';
import type { Settings } from '@/features/settings/types/settings';
import type { UpdateStatus } from '@/features/update/types/update';
import { AppVariant } from '@/shared/types/app';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke, Channel: class {} }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (event: string, callback: EventCallback<Status>): Promise<() => void> => {
    if (event === 'automation-status-changed') onAutomationStatus = callback;
    return () => {};
  },
}));
vi.mock('@/app/build', () => ({
  AppVariant,
  appVariant: AppVariant.Release,
  appVersion: '1.2.0',
}));
vi.mock('@/shared/i18n/toast', () => ({ useTranslatedToast: () => ({ add: vi.fn() }) }));

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
  backendStatus = {
    operation: 'idle',
    availableUpdate: updateInfo,
    pendingUpdate: null,
  };
  saveResult = Promise.resolve();
  invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case 'load_oea_settings':
        return {
          uiLocale: 'en-US',
          majorVersion: 0,
          minorVersion: 3,
          minimizeToTray: false,
          soundVolume: 0.5,
          updateSource: 'mirrorchyan',
          mirrorchyanCdkEncrypted: '',
          updateProxyMode: 'system',
          updateProxyUrl: '',
          autoDownloadUpdates: false,
          scanTipsDismissedVersion: 0,
        } satisfies Settings;
      case 'save_oea_settings':
        return saveResult;
      case 'get_automation_status':
        return { state: 'idle', lastRun: null };
      case 'get_update_status':
        return backendStatus;
      case 'check_update':
        backendStatus = { ...backendStatus, availableUpdate: updateInfo };
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

describe('更新安装确认', () => {
  it('自动下载完成及重载前端后都等待确认', async () => {
    settings.editSettings({ autoDownloadUpdates: true });
    await settle();
    await update.checkUpdate('automatic');
    expect(update.pendingUpdate.value).toEqual(updateInfo);
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    vi.resetModules();
    settings = await import('@/features/settings/settings');
    await settings.initOeaSettings();
    update = await import('./update');
    await update.initUpdateState();
    await settle();
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    expect(update.pendingUpdate.value).toEqual(updateInfo);
    const { useUpdateAction } = await import('./useUpdateAction');
    await useUpdateAction().activate();
    expect(update.installConfirmation.value?.kind).toBe('online');
  });

  it('手动检查仍自动下载，但取消确认不安装，确认才安装并重启', async () => {
    settings.editSettings({ autoDownloadUpdates: true });
    await settle();
    backendStatus = {
      operation: 'idle',
      availableUpdate: null,
      pendingUpdate: null,
    };
    await update.refreshUpdateStatus();
    const { useUpdateAction } = await import('./useUpdateAction');
    const action = useUpdateAction();
    await action.activate();
    await settle();
    expect(invoke.mock.calls.some(([command]) => command === 'download_update')).toBe(true);
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    await action.activate();
    expect(action.confirmation.value?.kind).toBe('online');
    await action.cancel();
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    await action.activate();
    void action.confirm();
    await settle();
    expect(invoke).toHaveBeenCalledWith('install_update');
  });

  it('手动更新在任务运行时不能安装，任务结束后仍等待用户操作', async () => {
    await update.startDownload();
    const { useUpdateAction } = await import('./useUpdateAction');
    const action = useUpdateAction();
    await publishAutomationStatus({ state: 'running', taskKind: 'archiveScan' });
    await action.activate();
    expect(action.confirmation.value).toBeNull();
    await publishAutomationStatus({ state: 'idle', lastRun: null });
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    await action.activate();
    await publishAutomationStatus({ state: 'running', taskKind: 'archiveScan' });
    await action.confirm();
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    expect(action.confirmation.value?.kind).toBe('online');
  });

  it('安装准备失败后可以重新下载并再次确认', async () => {
    await update.startDownload();
    const { useUpdateAction } = await import('./useUpdateAction');
    const action = useUpdateAction();
    await action.activate();
    const previous = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command: string) => {
      if (command === 'install_update') {
        backendStatus = { ...backendStatus, pendingUpdate: null };
        throw new Error('helper launch failed');
      }
      return previous(command);
    });
    await action.confirm();
    expect(action.confirmation.value).toBeNull();
    expect(action.phase.value).toBe('installFailed');
    expect(action.disabledReason.value).toBe('');
    invoke.mockImplementation(previous);
    await action.activate();
    await action.activate();
    expect(action.confirmation.value?.kind).toBe('online');
  });

  it.each([false, true])(
    '后端 Busy 后保留安装确认并允许重试（本地包：%s）',
    async (local: boolean) => {
      let busy = true;
      const previous = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
        if (command === 'select_developer_update') return true;
        if (command === 'install_update' || command === 'developer_install_update') {
          if (busy) throw { reason: { kind: 'busy' } };
          return new Promise<void>(() => {});
        }
        return previous(command, args);
      });
      const { useUpdateAction } = await import('./useUpdateAction');
      const action = useUpdateAction();
      if (local) await update.selectLocalUpdate();
      else {
        await update.startDownload();
        await action.activate();
      }
      await action.confirm();
      expect(action.confirmation.value?.kind).toBe(local ? 'local' : 'online');
      expect(update.localPackageSelected.value).toBe(local);
      busy = false;
      void action.confirm();
      await settle();
      const command = local ? 'developer_install_update' : 'install_update';
      expect(invoke.mock.calls.filter(([name]) => name === command)).toHaveLength(2);
      expect(invoke.mock.calls.filter(([name]) => name === 'select_developer_update')).toHaveLength(
        local ? 1 : 0,
      );
    },
  );

  it('取消本地包确认后，旧确认无法安装，重新选择失败后仍可重试', async () => {
    const { useUpdateAction } = await import('./useUpdateAction');
    const action = useUpdateAction();
    const previous = invoke.getMockImplementation()!;
    let selectionFails = false;
    invoke.mockImplementation(async (command: string) => {
      if (command === 'select_developer_update') {
        if (selectionFails) throw new Error('cannot read ZIP');
        return true;
      }
      if (command === 'discard_developer_update') return;
      return previous(command);
    });
    await update.selectLocalUpdate();
    expect(action.confirmation.value?.kind).toBe('local');
    await action.cancel();
    await action.confirm();
    expect(invoke.mock.calls.some(([command]) => command === 'developer_install_update')).toBe(
      false,
    );
    selectionFails = true;
    await update.selectLocalUpdate();
    expect(action.confirmation.value).toBeNull();
    expect(update.updateOperationBusy.value).toBe(false);
    selectionFails = false;
    await update.selectLocalUpdate();
    expect(action.confirmation.value?.kind).toBe('local');
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
    '扫描$name后保留已下载更新，直到用户确认',
    async ({ outcome }) => {
      settings.editSettings({ autoDownloadUpdates: true });
      await settle();
      await publishAutomationStatus({ state: 'running', taskKind: 'archiveScan' });
      await update.checkUpdate('automatic');
      await settle();
      expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
      invoke.mockClear();
      await publishAutomationStatus({ state: 'stopping', taskKind: 'archiveScan' });
      expect(invoke).not.toHaveBeenCalled();
      await publishAutomationStatus({
        state: 'idle',
        lastRun: { taskKind: 'archiveScan', outcome },
      });
      expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
      const { useUpdateAction } = await import('./useUpdateAction');
      const action = useUpdateAction();
      await action.activate();
      expect(action.confirmation.value?.kind).toBe('online');
      await action.cancel();
      expect(update.pendingUpdate.value).toEqual(updateInfo);
      expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
    },
  );

  it('自动下载使用已保存设置，关闭后保留已下载更新', async () => {
    let completeSave!: () => void;
    saveResult = new Promise<void>((resolve) => {
      completeSave = resolve;
    });
    settings.editSettings({ autoDownloadUpdates: true });
    await update.checkUpdate();
    expect(update.pendingUpdate.value).toBeNull();
    completeSave();
    await settle();
    await update.checkUpdate();
    expect(update.pendingUpdate.value).toEqual(updateInfo);
    settings.editSettings({ autoDownloadUpdates: false });
    await settle();
    expect(update.pendingUpdate.value).toEqual(updateInfo);
    expect(invoke.mock.calls.some(([command]) => command === 'install_update')).toBe(false);
  });
});
