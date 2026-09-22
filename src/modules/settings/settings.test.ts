import { describe, expect, it } from 'vitest';

import { UpdateSource } from './model';
import {
  CURRENT_SCAN_TIPS_VERSION,
  DEFAULT_OEA_CONFIG,
  type PersistedOeaConfig,
  type SettingsPersistence,
} from './persistence';
import { createSettingsModule } from './settings';

interface Deferred<Value> {
  promise: Promise<Value>;
  resolve(value: Value): void;
  reject(error: unknown): void;
}

class ControlledPersistence implements SettingsPersistence {
  readonly saves: Array<Readonly<PersistedOeaConfig>> = [];
  readonly saveDeferreds: Array<Deferred<void>> = [];
  readonly encryptInputs: string[] = [];
  readonly encryptDeferreds: Array<Deferred<string>> = [];
  maxConcurrentSaves = 0;
  concurrentSaves = 0;
  loadError: unknown | undefined;
  loadedConfig: PersistedOeaConfig = cloneConfig(DEFAULT_OEA_CONFIG);
  decryptedCdk = 'loaded-value';

  async load(): Promise<PersistedOeaConfig> {
    if (this.loadError !== undefined) {
      throw this.loadError;
    }
    return cloneConfig(this.loadedConfig);
  }

  save(candidate: Readonly<PersistedOeaConfig>): Promise<void> {
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
    return encrypted === '' ? '' : this.decryptedCdk;
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
    expect(settings.settingsStatus.kind).toBe('saving');
    persistence.saveDeferreds[0]?.reject(new Error('写入失败'));
    await flushMicrotasks();

    expect(settings.settingsDraft.autoDownloadUpdates).toBe(false);
    expect(settings.effectiveSettings.autoDownloadUpdates).toBe(true);
    expect(settings.settingsStatus.kind).toBe('save-error');
    expect(persistence.saves).toHaveLength(1);

    settings.retrySettingsSave();
    await flushMicrotasks();
    expect(persistence.saves).toHaveLength(2);
    expect(settings.settingsStatus.kind).toBe('saving');
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

  it('CDK 在候选内裁剪、复用已存密文并在清空后保存空密文', async () => {
    const persistence = new ControlledPersistence();
    persistence.loadedConfig = {
      ...DEFAULT_OEA_CONFIG,
      mirrorchyanCdkEncrypted: 'stored-ciphertext',
    };
    persistence.decryptedCdk = 'existing-key';
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    settings.settingsDraft.autoDownloadUpdates = false;
    await flushMicrotasks();
    expect(persistence.encryptInputs).toEqual([]);
    expect(persistence.saves[0]?.mirrorchyanCdkEncrypted).toBe('stored-ciphertext');
    persistence.saveDeferreds[0]?.resolve();
    await flushMicrotasks();

    settings.settingsDraft.mirrorchyanCdk = '  new-key  ';
    await flushMicrotasks();
    expect(persistence.encryptInputs).toEqual(['new-key']);
    persistence.encryptDeferreds[0]?.resolve('new-ciphertext');
    await flushMicrotasks();
    expect(persistence.saves[1]?.mirrorchyanCdkEncrypted).toBe('new-ciphertext');
    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();

    settings.settingsDraft.mirrorchyanCdk = '   ';
    await flushMicrotasks();
    expect(persistence.saves[2]?.mirrorchyanCdkEncrypted).toBe('');
    persistence.saveDeferreds[2]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.mirrorchyanCdk).toBe('');
  });

  it('保留 DTO 版本字段，并将扫描提示逻辑状态映射为持久化版本', async () => {
    const persistence = new ControlledPersistence();
    persistence.loadedConfig = {
      ...DEFAULT_OEA_CONFIG,
      majorVersion: 12,
      minorVersion: 34,
      scanTipsDismissedVersion: CURRENT_SCAN_TIPS_VERSION,
    };
    const settings = createSettingsModule(persistence);
    await settings.initializeSettings();

    expect(settings.effectiveSettings.scanGuideEnabled).toBe(false);
    settings.settingsDraft.scanGuideEnabled = true;
    await flushMicrotasks();
    expect(persistence.saves[0]).toMatchObject({
      majorVersion: 12,
      minorVersion: 34,
      scanTipsDismissedVersion: 0,
    });
    expect(settings.effectiveSettings.scanGuideEnabled).toBe(false);
    persistence.saveDeferreds[0]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.scanGuideEnabled).toBe(true);
    settings.settingsDraft.scanGuideEnabled = false;
    await flushMicrotasks();
    expect(persistence.saves[1]?.scanTipsDismissedVersion).toBe(CURRENT_SCAN_TIPS_VERSION);
    persistence.saveDeferreds[1]?.resolve();
    await flushMicrotasks();

    expect(settings.effectiveSettings.scanGuideEnabled).toBe(false);

    const newGuideVersionPersistence = new ControlledPersistence();
    newGuideVersionPersistence.loadedConfig = {
      ...DEFAULT_OEA_CONFIG,
      scanTipsDismissedVersion: 0,
    };
    const settingsWithNewGuideVersion = createSettingsModule(newGuideVersionPersistence);
    await settingsWithNewGuideVersion.initializeSettings();

    expect(settingsWithNewGuideVersion.effectiveSettings.scanGuideEnabled).toBe(true);
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

function cloneConfig(config: PersistedOeaConfig): PersistedOeaConfig {
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
