import { describe, expect, it } from 'vitest';

import { UpdateSource, type OeaConfig } from '@/types/oeaConfig';

import { DEFAULT_OEA_CONFIG, type SettingsPersistence } from './persistence';
import { createSettingsModule } from './settings';

interface Deferred<Value> {
  promise: Promise<Value>;
  resolve(value: Value): void;
  reject(error: unknown): void;
}

class ControlledPersistence implements SettingsPersistence {
  readonly saves: Array<Readonly<OeaConfig>> = [];
  readonly saveDeferreds: Array<Deferred<void>> = [];
  readonly encryptInputs: string[] = [];
  readonly encryptDeferreds: Array<Deferred<string>> = [];
  maxConcurrentSaves = 0;
  concurrentSaves = 0;
  loadError: unknown | undefined;
  loadedConfig: OeaConfig = cloneConfig(DEFAULT_OEA_CONFIG);

  async load(): Promise<OeaConfig> {
    if (this.loadError !== undefined) {
      throw this.loadError;
    }
    return cloneConfig(this.loadedConfig);
  }

  save(candidate: Readonly<OeaConfig>): Promise<void> {
    this.saves.push(candidate);
    this.concurrentSaves += 1;
    this.maxConcurrentSaves = Math.max(this.maxConcurrentSaves, this.concurrentSaves);
    const deferred = createDeferred<void>();
    this.saveDeferreds.push(deferred);
    return deferred.promise.finally(() => {
      this.concurrentSaves -= 1;
    });
  }

  encryptCdk(plain: string): Promise<string> {
    this.encryptInputs.push(plain);
    const deferred = createDeferred<string>();
    this.encryptDeferreds.push(deferred);
    return deferred.promise;
  }

  async decryptCdk(encrypted: string): Promise<string> {
    return encrypted === '' ? '' : 'loaded-value';
  }
}

describe('settings single writer', () => {
  it('保存不可变 candidate，并合并保存期间的中间编辑', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.soundVolume = 0.2;
    await flushMicrotasks();
    settings.settingsDraft.soundVolume = 0.9;
    settings.settingsDraft.soundVolume = 0.7;

    expect(persistence.saves).toHaveLength(1);
    expect(persistence.saves[0]?.soundVolume).toBe(0.2);
    expect(persistence.maxConcurrentSaves).toBe(1);

    persistence.saveDeferreds[0]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.soundVolume).toBe(0.2);
    expect(persistence.saves).toHaveLength(2);
    expect(persistence.saves[1]?.soundVolume).toBe(0.7);

    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.soundVolume).toBe(0.7);
    expect(settings.settingsStatus.kind).toBe('idle');
  });

  it('保存失败时保留 draft 和旧 effective，且只在显式重试后再次保存', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.autoDownloadUpdates = false;
    await flushMicrotasks();
    persistence.saveDeferreds[0]?.reject(new Error('写入失败'));
    await flushMicrotasks();

    expect(settings.settingsDraft.autoDownloadUpdates).toBe(false);
    expect(settings.effectiveSettings.autoDownloadUpdates).toBe(true);
    expect(settings.settingsStatus.kind).toBe('save-error');
    expect(persistence.saves).toHaveLength(1);

    settings.retrySettingsSave();
    await flushMicrotasks();
    expect(persistence.saves).toHaveLength(2);
    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.autoDownloadUpdates).toBe(false);
    expect(settings.settingsStatus.kind).toBe('idle');
  });

  it('旧 candidate 失败后会尝试最新 draft；最新失败后停止', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.updateSource = UpdateSource.Oem;
    await flushMicrotasks();
    settings.settingsDraft.updateSource = UpdateSource.Github;
    persistence.saveDeferreds[0]?.reject(new Error('首次失败'));
    await flushMicrotasks();

    expect(persistence.saves).toHaveLength(2);
    expect(persistence.saves[1]?.updateSource).toBe(UpdateSource.Github);

    persistence.saveDeferreds[1]?.reject(new Error('最终失败'));
    await flushMicrotasks();

    expect(settings.settingsDraft.updateSource).toBe(UpdateSource.Github);
    expect(settings.effectiveSettings.updateSource).toBe(UpdateSource.Mirrorchyan);
    expect(settings.settingsStatus.kind).toBe('save-error');
    expect(persistence.saves).toHaveLength(2);
  });

  it('恢复操作将完整 draft 重置到最近 effective，并清除保存错误', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.updateProxyUrl = 'https://example.invalid';
    await flushMicrotasks();
    persistence.saveDeferreds[0]?.reject(new Error('写入失败'));
    await flushMicrotasks();

    settings.discardSettingsDraft();
    await flushMicrotasks();

    expect(settings.settingsDraft.updateProxyUrl).toBe('');
    expect(settings.settingsStatus.kind).toBe('idle');
    expect(persistence.saves).toHaveLength(1);
  });

  it('保存完成附近的编辑会在 writer 停止前被再次捕获', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.minimizeToTray = true;
    await flushMicrotasks();
    persistence.saveDeferreds[0]?.resolve();
    settings.settingsDraft.minimizeToTray = false;
    await flushMicrotasks();

    expect(persistence.saves).toHaveLength(2);
    expect(persistence.saves[1]?.minimizeToTray).toBe(false);

    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();
    expect(settings.effectiveSettings.minimizeToTray).toBe(false);
  });

  it('CDK 加密与保存按 candidate 串行，较新的编辑不会改写在途候选', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.mirrorchyanCdk = 'alpha';
    await flushMicrotasks();
    settings.settingsDraft.mirrorchyanCdk = 'beta';

    expect(persistence.encryptInputs).toEqual(['alpha']);
    persistence.encryptDeferreds[0]?.resolve('encrypted-alpha');
    await flushMicrotasks();
    expect(persistence.saves[0]?.mirrorchyanCdkEncrypted).toBe('encrypted-alpha');

    persistence.saveDeferreds[0]?.resolve();
    await flushMicrotasks();
    expect(persistence.encryptInputs).toEqual(['alpha', 'beta']);

    persistence.encryptDeferreds[1]?.resolve('encrypted-beta');
    await flushMicrotasks();
    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.mirrorchyanCdk).toBe('beta');
  });

  it('音量拒绝非有限值，并将有限数字限制在合法范围', async () => {
    const persistence = new ControlledPersistence();
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    (settings.settingsDraft as { soundVolume: unknown }).soundVolume = [0.2];
    (settings.settingsDraft as { soundVolume: unknown }).soundVolume = Number.NaN;
    (settings.settingsDraft as { soundVolume: unknown }).soundVolume = Number.POSITIVE_INFINITY;
    expect(settings.settingsDraft.soundVolume).toBe(0.5);

    settings.settingsDraft.soundVolume = 2;
    await flushMicrotasks();
    expect(settings.settingsDraft.soundVolume).toBe(1);
    expect(persistence.saves[0]?.soundVolume).toBe(1);
  });

  it('加载失败时保留现有默认回退并发布加载错误状态', async () => {
    const persistence = new ControlledPersistence();
    persistence.loadError = new Error('读取失败');
    const settings = createSettingsModule(persistence);

    await settings.initializeSettings();

    expect(settings.effectiveSettings.updateSource).toBe(UpdateSource.Mirrorchyan);
    expect(settings.settingsStatus.kind).toBe('load-error');
  });
});

function cloneConfig(config: OeaConfig): OeaConfig {
  return { ...config };
}

function createDeferred<Value>(): Deferred<Value> {
  let resolve: (value: Value) => void = () => undefined;
  let reject: (error: unknown) => void = () => undefined;
  const promise = new Promise<Value>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}
