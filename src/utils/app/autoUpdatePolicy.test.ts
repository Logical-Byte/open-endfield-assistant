import { describe, expect, it } from 'vitest';

import type { PersistedOeaConfig, SettingsPersistence } from '@/modules/settings/persistence';
import { DEFAULT_OEA_CONFIG } from '@/modules/settings/persistence';
import { createSettingsModule } from '@/modules/settings/settings';

import { decideAutomaticUpdateAction } from './autoUpdatePolicy';

class DeferredSavePersistence implements SettingsPersistence {
  readonly saveDeferred = createDeferred<void>();

  async load(): Promise<PersistedOeaConfig> {
    return { ...DEFAULT_OEA_CONFIG };
  }

  save(): Promise<void> {
    return this.saveDeferred.promise;
  }

  async encryptCdk(plain: string): Promise<string> {
    return plain;
  }

  async decryptCdk(encrypted: string): Promise<string> {
    return encrypted;
  }
}

interface Deferred<Value> {
  promise: Promise<Value>;
  resolve(value: Value): void;
}

describe('自动更新编排设置生效时机', () => {
  it('保存成功前沿用旧设置，成功后才停止自动下载和安装', async () => {
    const persistence = new DeferredSavePersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.autoDownloadUpdates = false;
    settings.settingsDraft.autoInstallUpdates = false;
    await flushMicrotasks();

    expect(decideAutomaticUpdateAction(settings.effectiveSettings, 'available')).toBe('download');
    expect(decideAutomaticUpdateAction(settings.effectiveSettings, 'pending')).toBe('install');

    persistence.saveDeferred.resolve();
    await flushMicrotasks();

    expect(decideAutomaticUpdateAction(settings.effectiveSettings, 'available')).toBe('none');
    expect(decideAutomaticUpdateAction(settings.effectiveSettings, 'pending')).toBe('none');
  });
});

function createDeferred<Value>(): Deferred<Value> {
  let resolve: (value: Value) => void = () => undefined;
  const promise = new Promise<Value>((promiseResolve) => {
    resolve = promiseResolve;
  });
  return { promise, resolve };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}
