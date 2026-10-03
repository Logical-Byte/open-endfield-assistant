import { describe, expect, it, vi } from 'vitest';

import { UpdateProxyMode, UpdateSource, type Settings } from '@/features/settings/types/settings';
import { createSettingsStore, type SettingsState } from './settingsStore';

type ReadySettingsState = Extract<SettingsState, { status: 'ready' }>;

function createSettingsFixture(overrides: Partial<Settings> = {}): Settings {
  return {
    majorVersion: 0,
    minorVersion: 0,
    minimizeToTray: false,
    soundVolume: 0.5,
    updateSource: UpdateSource.Mirrorchyan,
    mirrorchyanCdkEncrypted: '',
    updateProxyMode: UpdateProxyMode.System,
    updateProxyUrl: '',
    autoDownloadUpdates: true,
    autoInstallUpdates: true,
    scanTipsDismissedVersion: 0,
    essenceScan: {
      autoMark: false,
      nonFiveStar: 'process',
      protectLocked: true,
      skipAbandoned: false,
      highLevel: null,
      excludedWeaponIds: [],
      customKeeps: [],
    },
    ...overrides,
  };
}

function ready(store: ReturnType<typeof createSettingsStore>): ReadySettingsState {
  const state = store.state.value;
  expect(state.status).toBe('ready');
  if (state.status !== 'ready') throw new Error(`预期 ready，实际为 ${state.status}`);
  return state;
}

function deferred<T>(): {
  promise: Promise<T>;
  resolve(value: T): void;
  reject(error: Error): void;
} {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function setup(settings: Settings = createSettingsFixture()): {
  store: ReturnType<typeof createSettingsStore>;
  io: {
    load: ReturnType<typeof vi.fn<() => Promise<Settings>>>;
    save: ReturnType<typeof vi.fn<(settings: Settings) => Promise<void>>>;
    encrypt: ReturnType<typeof vi.fn<(plain: string) => Promise<string>>>;
    decrypt: ReturnType<typeof vi.fn<(encrypted: string) => Promise<string>>>;
  };
} {
  const io = {
    load: vi.fn<() => Promise<Settings>>().mockResolvedValue(settings),
    save: vi.fn<(settings: Settings) => Promise<void>>().mockResolvedValue(undefined),
    encrypt: vi.fn<(plain: string) => Promise<string>>().mockResolvedValue('test-ciphertext'),
    decrypt: vi.fn<(encrypted: string) => Promise<string>>().mockResolvedValue('test-plain'),
  };
  return { store: createSettingsStore(io), io };
}

describe('设置提交', () => {
  it.each([
    { value: -0.2, expected: 0 },
    { value: 1.5, expected: 1 },
    { value: 0.123, expected: 0.123 },
  ])('音量编辑 $value 保存为 $expected，不按滑块步长舍入', async ({ value, expected }) => {
    const { store, io } = setup();
    await store.initialize();

    store.edit({ soundVolume: value });

    expect(ready(store).draft.soundVolume).toBe(expected);
    expect(io.save.mock.calls[0]?.[0].soundVolume).toBe(expected);
    await Promise.resolve();
    expect(ready(store).effective.soundVolume).toBe(expected);
  });

  it.each([NaN, Infinity, -Infinity])(
    '忽略非有限音量 %s，同次编辑的其他字段仍保存',
    async (value) => {
      const { store, io } = setup();
      await store.initialize();

      store.edit({ soundVolume: value, minimizeToTray: true });

      expect(ready(store).draft).toMatchObject({ soundVolume: 0.5, minimizeToTray: true });
      expect(io.save.mock.calls[0]?.[0]).toMatchObject({ soundVolume: 0.5, minimizeToTray: true });
      await Promise.resolve();
      expect(ready(store).effective).toMatchObject({ soundVolume: 0.5, minimizeToTray: true });
    },
  );

  it('首次从后端初始化完成前不暴露任何设置值', () => {
    const { store, io } = setup();

    expect(store.state.value).toEqual({ status: 'initializing' });
    expect(io.load).not.toHaveBeenCalled();
  });

  it('纯浏览器模式明确标记后端设置不可用', () => {
    const { store, io } = setup();

    store.markUnsupported();

    expect(store.state.value).toEqual({
      status: 'unavailable',
      reason: { type: 'unsupported' },
    });
    expect(io.load).not.toHaveBeenCalled();
  });

  it('扫描提示按版本展示，无关编辑保留旧版本号，重新开启后可再次确认', async () => {
    const { store, io } = setup(createSettingsFixture({ scanTipsDismissedVersion: 7 }));
    await store.initialize();
    expect(ready(store).draft.scanGuideEnabled).toBe(false);
    store.edit({ soundVolume: 0.8 });
    await Promise.resolve();
    expect(io.save.mock.calls[0]?.[0].scanTipsDismissedVersion).toBe(7);
    store.edit({ scanGuideEnabled: true });
    await Promise.resolve();
    expect(io.save.mock.calls[1]?.[0].scanTipsDismissedVersion).toBe(0);
    expect(ready(store).effective.scanGuideEnabled).toBe(true);
    store.edit({ scanGuideEnabled: false });
    await Promise.resolve();
    const saved = io.save.mock.calls[2]![0];
    expect(saved.scanTipsDismissedVersion).toBeGreaterThan(0);
    const restarted = setup(saved).store;
    await restarted.initialize();
    expect(ready(restarted).effective.scanGuideEnabled).toBe(false);
    const firstRun = setup().store;
    await firstRun.initialize();
    expect(ready(firstRun).effective.scanGuideEnabled).toBe(true);
  });

  it('初始化期间不接受编辑，初始化失败也不会用占位默认值覆盖已有设置', async () => {
    const { store, io } = setup();
    const load = deferred<Settings>();
    io.load.mockReturnValueOnce(load.promise);
    const initialization = store.initialize();
    store.edit({ soundVolume: 0.1 });
    expect(store.state.value).toEqual({ status: 'initializing' });
    expect(io.save).not.toHaveBeenCalled();
    load.reject(new Error('IPC 失败'));
    await initialization;
    store.edit({ minimizeToTray: true });
    expect(store.state.value.status).toBe('unavailable');
    if (store.state.value.status !== 'unavailable') throw new Error('预期 unavailable');
    expect(store.state.value.reason.type).toBe('initialize-error');
    if (store.state.value.reason.type !== 'initialize-error') {
      throw new Error('预期 initialize-error');
    }
    expect(store.state.value.reason.error.message).toBe('IPC 失败');
    expect(io.save).not.toHaveBeenCalled();

    io.load.mockResolvedValueOnce(createSettingsFixture({ soundVolume: 0.8 }));
    await store.initialize();
    expect(ready(store).draft.soundVolume).toBe(0.8);
    expect(ready(store).effective.soundVolume).toBe(0.8);
  });

  it('捕获每次候选，合并在途编辑，保存成功后才发布同一候选', async () => {
    const { store, io } = setup();
    await store.initialize();
    const first = deferred<void>();
    const second = deferred<void>();
    io.save.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    store.edit({ autoDownloadUpdates: false, soundVolume: 0.2 });
    store.edit({ soundVolume: 0.9 });
    store.edit({ soundVolume: 0.7 });
    expect(io.save).toHaveBeenCalledTimes(1);
    expect(io.save.mock.calls[0]?.[0].soundVolume).toBe(0.2);
    expect(ready(store).effective.autoDownloadUpdates).toBe(true);

    first.resolve();
    await first.promise;
    expect(ready(store).effective.autoDownloadUpdates).toBe(false);
    expect(ready(store).effective.soundVolume).toBe(0.2);
    expect(io.save).toHaveBeenCalledTimes(2);
    expect(io.save.mock.calls[1]?.[0].soundVolume).toBe(0.7);
    second.resolve();
    await second.promise;
    expect(ready(store).effective.soundVolume).toBe(0.7);
  });

  it('旧候选失败继续最新编辑，最新失败保留草稿并停止，显式重试后生效', async () => {
    const { store, io } = setup();
    await store.initialize();
    const first = deferred<void>();
    const second = deferred<void>();
    io.save.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    store.edit({ soundVolume: 0.2 });
    store.edit({ soundVolume: 0.7, autoInstallUpdates: false });
    first.reject(new Error('首次失败'));
    await first.promise.catch(() => {});
    expect(io.save).toHaveBeenCalledTimes(2);
    second.reject(new Error('磁盘写入失败'));
    await second.promise.catch(() => {});
    expect(ready(store).draft.soundVolume).toBe(0.7);
    expect(ready(store).effective.soundVolume).toBe(0.5);
    expect(ready(store).effective.autoInstallUpdates).toBe(true);
    expect(ready(store).saveError?.message).toBe('磁盘写入失败');
    expect(io.save).toHaveBeenCalledTimes(2);

    store.retry();
    await Promise.resolve();
    expect(io.save).toHaveBeenCalledTimes(3);
    expect(ready(store).effective.soundVolume).toBe(0.7);
    expect(ready(store).effective.autoInstallUpdates).toBe(false);
    expect(ready(store).saveError).toBeNull();
  });

  it('加密与保存处于同一队列，等待期间的新 CDK 不能被旧密文覆盖', async () => {
    const { store, io } = setup();
    await store.initialize();
    const encryption = deferred<string>();
    const save = deferred<void>();
    io.encrypt.mockReturnValueOnce(encryption.promise).mockResolvedValueOnce('cipher-beta');
    io.save.mockReturnValueOnce(save.promise);
    store.edit({ mirrorchyanCdk: '  alpha  ' });
    store.edit({ mirrorchyanCdk: 'beta' });
    expect(io.encrypt.mock.calls).toEqual([['alpha']]);
    expect(io.save).not.toHaveBeenCalled();
    encryption.resolve('cipher-alpha');
    await encryption.promise;
    expect(io.save.mock.calls[0]?.[0].mirrorchyanCdkEncrypted).toBe('cipher-alpha');
    expect(io.encrypt).toHaveBeenCalledTimes(1);
    save.resolve();
    await save.promise;
    expect(io.encrypt.mock.calls).toEqual([['alpha'], ['beta']]);
    await Promise.resolve();
    expect(io.save.mock.calls[1]?.[0].mirrorchyanCdkEncrypted).toBe('cipher-beta');
    await Promise.resolve();
    expect(ready(store).effective.mirrorchyanCdk).toBe('beta');
  });

  it('加密失败保留有效 CDK，后续编辑可以重新提交，未修改 CDK 时复用密文', async () => {
    const { store, io } = setup(createSettingsFixture({ mirrorchyanCdkEncrypted: 'old-cipher' }));
    io.decrypt.mockResolvedValueOnce('  test-plain  ');
    await store.initialize();
    store.edit({ soundVolume: 0.6 });
    await Promise.resolve();
    expect(io.encrypt).not.toHaveBeenCalled();
    expect(io.save.mock.calls[0]?.[0].mirrorchyanCdkEncrypted).toBe('old-cipher');
    io.save.mockClear();
    io.encrypt.mockRejectedValueOnce(new Error('加密失败'));
    store.edit({ mirrorchyanCdk: 'new-plain' });
    await Promise.resolve();
    expect(io.save).not.toHaveBeenCalled();
    expect(ready(store).effective.mirrorchyanCdk).toBe('  test-plain  ');
    expect(ready(store).saveError?.message).toBe('加密失败');
    store.edit({ soundVolume: 0.7 });
    await Promise.resolve();
    await Promise.resolve();
    expect(ready(store).effective.mirrorchyanCdk).toBe('new-plain');
    store.edit({ soundVolume: 0.8 });
    await Promise.resolve();
    expect(io.encrypt).toHaveBeenCalledTimes(2);
    expect(io.save.mock.calls[1]?.[0].mirrorchyanCdkEncrypted).toBe('test-ciphertext');
  });

  it.each(['', 'replacement'])(
    '解密失败保留原密文，显式写入 %j 才清空或替换',
    async (replacement) => {
      const { store, io } = setup(createSettingsFixture({ mirrorchyanCdkEncrypted: 'old-cipher' }));
      io.decrypt.mockRejectedValueOnce(new Error('解密失败'));
      await store.initialize();
      expect(ready(store).draft.mirrorchyanCdk).toBeNull();
      store.edit({ minimizeToTray: true });
      await Promise.resolve();
      expect(io.save.mock.calls[0]?.[0].mirrorchyanCdkEncrypted).toBe('old-cipher');
      expect(io.encrypt).not.toHaveBeenCalled();
      store.edit({ mirrorchyanCdk: replacement });
      await Promise.resolve();
      await Promise.resolve();
      expect(io.save.mock.calls[1]?.[0].mirrorchyanCdkEncrypted).toBe(
        replacement ? 'test-ciphertext' : '',
      );
      expect(ready(store).effective.mirrorchyanCdk).toBe(replacement);
    },
  );
});
