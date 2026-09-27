import { describe, expect, it, vi } from 'vitest';

import type { OeaConfig } from '@/types/oeaConfig';
import { DEFAULT_OEA_CONFIG } from './config';
import { createConfigStore } from './configStore';

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

function setup(config: OeaConfig = { ...DEFAULT_OEA_CONFIG }): {
  store: ReturnType<typeof createConfigStore>;
  io: {
    load: ReturnType<typeof vi.fn<() => Promise<OeaConfig>>>;
    save: ReturnType<typeof vi.fn<(config: OeaConfig) => Promise<void>>>;
    encrypt: ReturnType<typeof vi.fn<(plain: string) => Promise<string>>>;
    decrypt: ReturnType<typeof vi.fn<(encrypted: string) => Promise<string>>>;
  };
} {
  const io = {
    load: vi.fn<() => Promise<OeaConfig>>().mockResolvedValue(config),
    save: vi.fn<(config: OeaConfig) => Promise<void>>().mockResolvedValue(undefined),
    encrypt: vi.fn<(plain: string) => Promise<string>>().mockResolvedValue('test-ciphertext'),
    decrypt: vi.fn<(encrypted: string) => Promise<string>>().mockResolvedValue('test-plain'),
  };
  return { store: createConfigStore(DEFAULT_OEA_CONFIG, io), io };
}

describe('设置提交', () => {
  it('加载期间不接受编辑，加载失败也不会用占位默认值覆盖已有配置', async () => {
    const { store, io } = setup();
    const load = deferred<OeaConfig>();
    io.load.mockReturnValueOnce(load.promise);
    const initialization = store.initialize();
    store.edit({ soundVolume: 0.1 });
    expect(store.loading.value).toBe(true);
    expect(io.save).not.toHaveBeenCalled();
    load.reject(new Error('IPC 失败'));
    await initialization;
    store.edit({ minimizeToTray: true });
    expect(store.loaded.value).toBe(false);
    expect(store.loadError.value?.message).toBe('IPC 失败');
    expect(io.save).not.toHaveBeenCalled();

    io.load.mockResolvedValueOnce({ ...DEFAULT_OEA_CONFIG, soundVolume: 0.8 });
    await store.initialize();
    expect(store.loaded.value).toBe(true);
    expect(store.loadError.value).toBeNull();
    expect(store.draft.value.soundVolume).toBe(0.8);
    expect(store.effective.value.soundVolume).toBe(0.8);
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
    expect(store.effective.value.autoDownloadUpdates).toBe(true);
    expect(store.saving.value).toBe(true);

    first.resolve();
    await first.promise;
    expect(store.effective.value.autoDownloadUpdates).toBe(false);
    expect(store.effective.value.soundVolume).toBe(0.2);
    expect(io.save).toHaveBeenCalledTimes(2);
    expect(io.save.mock.calls[1]?.[0].soundVolume).toBe(0.7);
    expect(store.saving.value).toBe(true);
    second.resolve();
    await second.promise;
    expect(store.effective.value.soundVolume).toBe(0.7);
    expect(store.saving.value).toBe(false);
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
    expect(store.draft.value.soundVolume).toBe(0.7);
    expect(store.effective.value.soundVolume).toBe(0.5);
    expect(store.effective.value.autoInstallUpdates).toBe(true);
    expect(store.saveError.value?.message).toBe('磁盘写入失败');
    expect(store.saving.value).toBe(false);
    expect(io.save).toHaveBeenCalledTimes(2);

    store.retry();
    await Promise.resolve();
    expect(io.save).toHaveBeenCalledTimes(3);
    expect(store.effective.value.soundVolume).toBe(0.7);
    expect(store.effective.value.autoInstallUpdates).toBe(false);
    expect(store.saveError.value).toBeNull();
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
    expect(store.effective.value.mirrorchyanCdk).toBe('beta');
  });

  it('加密失败保留有效 CDK，后续编辑可以重新提交；未修改 CDK 时复用密文', async () => {
    const { store, io } = setup({ ...DEFAULT_OEA_CONFIG, mirrorchyanCdkEncrypted: 'old-cipher' });
    await store.initialize();
    io.encrypt.mockRejectedValueOnce(new Error('加密失败'));
    store.edit({ mirrorchyanCdk: 'new-plain' });
    await Promise.resolve();
    expect(io.save).not.toHaveBeenCalled();
    expect(store.effective.value.mirrorchyanCdk).toBe('test-plain');
    expect(store.saveError.value?.message).toBe('加密失败');
    store.edit({ soundVolume: 0.7 });
    await Promise.resolve();
    await Promise.resolve();
    expect(store.effective.value.mirrorchyanCdk).toBe('new-plain');
    store.edit({ soundVolume: 0.8 });
    await Promise.resolve();
    expect(io.encrypt).toHaveBeenCalledTimes(2);
    expect(io.save.mock.calls[1]?.[0].mirrorchyanCdkEncrypted).toBe('test-ciphertext');
  });

  it.each(['', 'replacement'])(
    '解密失败保留原密文，显式写入 %j 才清空或替换',
    async (replacement) => {
      const { store, io } = setup({ ...DEFAULT_OEA_CONFIG, mirrorchyanCdkEncrypted: 'old-cipher' });
      io.decrypt.mockRejectedValueOnce(new Error('解密失败'));
      await store.initialize();
      expect(store.draft.value.mirrorchyanCdk).toBeNull();
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
      expect(store.effective.value.mirrorchyanCdk).toBe(replacement);
    },
  );
});
