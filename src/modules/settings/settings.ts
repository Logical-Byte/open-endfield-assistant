import { reactive, readonly, type DeepReadonly } from 'vue';

import {
  UpdateProxyMode,
  UpdateSource,
  type SettingsDraft,
  type SettingsSnapshot,
  type SettingsStatus,
} from './model';
import {
  createDefaultSettingsDraft,
  DEFAULT_OEA_CONFIG,
  persistedFromSettingsDraft,
  settingsDraftFromPersisted,
  type PersistedOeaConfig,
  type SettingsPersistence,
} from './persistence';

/** 一个隔离的 settings 事务实例及其公开操作。 */
export interface SettingsModule {
  /** 用户当前编辑意图；写入会启动或唤醒 single writer。 */
  settingsDraft: SettingsDraft;
  /** 最近一次成功保存的只读逻辑设置。 */
  effectiveSettings: DeepReadonly<SettingsSnapshot>;
  /** 当前初始化、保存或错误状态。 */
  settingsStatus: DeepReadonly<SettingsStatus>;
  /** 加载持久化配置并启动待处理编辑的保存。 */
  initializeSettings(): Promise<void>;
  /** 重新提交当前失败的 draft。 */
  retrySettingsSave(): void;
  /** 将完整 draft 恢复为最近的 effective 设置。 */
  discardSettingsDraft(): void;
}

type MutableSettingsStatus = {
  kind: SettingsStatus['kind'];
  error?: unknown;
};

interface SettingsCandidate {
  settings: Readonly<SettingsDraft>;
  hasCdkIntentWhileUnknown: boolean;
}

/**
 * 建立独立的 settings 事务实例。
 *
 * 此 factory 是模块内部的测试接口；生产环境只通过 index.ts 中的单例访问。
 */
export function createSettingsModule(persistence: SettingsPersistence): SettingsModule {
  const rawDraft = reactive<SettingsDraft>(createDefaultSettingsDraft());
  const rawEffective = reactive<SettingsDraft>(createDefaultSettingsDraft());
  const rawStatus = reactive<MutableSettingsStatus>({ kind: 'loading' });

  let initialized = false;
  let initialization: Promise<void> | undefined;
  let writerRunning = false;
  let draftRevision = 0;
  let savedRevision = 0;
  let failedRevision: number | undefined;
  let lastPersisted: PersistedOeaConfig = clonePersisted(DEFAULT_OEA_CONFIG);
  let cdkPlaintextKnown = false;
  let cdkDecryptionError: unknown | undefined;
  // 明文未知时，空 draft 只是安全展示；只有显式 CDK 编辑才能修改加载的密文。
  let hasCdkIntentWhileUnknown = false;
  const initializationEdits: Partial<SettingsDraft> = {};

  const settingsDraft = createDraftProxy(rawDraft, edit);

  function edit<Key extends keyof SettingsDraft>(key: Key, value: SettingsDraft[Key]): void {
    const normalized = normalizeSetting(key, value);
    if (normalized === undefined) {
      return;
    }

    const isInitializationEdit = !initialized;
    const needsCdkIntent = key === 'mirrorchyanCdk' && !cdkPlaintextKnown;
    if (!isInitializationEdit && !needsCdkIntent && rawDraft[key] === normalized) {
      return;
    }

    rawDraft[key] = normalized as SettingsDraft[Key];
    if (isInitializationEdit) {
      initializationEdits[key] = normalized;
    }
    if (key === 'mirrorchyanCdk' && !cdkPlaintextKnown) {
      hasCdkIntentWhileUnknown = true;
    }
    draftRevision += 1;
    failedRevision = undefined;
    ensureWriter();
  }

  function ensureWriter(): void {
    if (!initialized || writerRunning || !hasPendingWork()) {
      return;
    }

    writerRunning = true;
    void runWriter();
  }

  function hasPendingWork(): boolean {
    return draftRevision > savedRevision && failedRevision !== draftRevision;
  }

  function readyStatus(): SettingsStatus {
    if (!cdkPlaintextKnown) {
      return { kind: 'decrypt-error', error: cdkDecryptionError };
    }
    return { kind: 'idle' };
  }

  async function runWriter(): Promise<void> {
    try {
      while (hasPendingWork()) {
        const targetRevision = draftRevision;
        const candidate = createCandidate(rawDraft, hasCdkIntentWhileUnknown);
        replaceStatus(rawStatus, { kind: 'saving' });

        try {
          const encryptedCdk = await encryptCandidateCdk(candidate);
          const persistedCandidate = Object.freeze(
            persistedFromSettingsDraft(candidate.settings, lastPersisted, encryptedCdk),
          );
          await persistence.save(persistedCandidate);

          lastPersisted = clonePersisted(persistedCandidate);
          replaceSettings(rawEffective, candidate.settings);
          if (!cdkPlaintextKnown && candidate.hasCdkIntentWhileUnknown) {
            cdkPlaintextKnown = true;
            cdkDecryptionError = undefined;
            hasCdkIntentWhileUnknown = false;
          }
          savedRevision = targetRevision;
          failedRevision = undefined;
        } catch (error) {
          if (draftRevision === targetRevision) {
            failedRevision = targetRevision;
            replaceStatus(rawStatus, { kind: 'save-error', error });
            return;
          }
        }
      }

      replaceStatus(rawStatus, readyStatus());
    } finally {
      writerRunning = false;
      // edit 与 finally 之间可以交错；重新检查避免遗漏最后一次唤醒。
      ensureWriter();
    }
  }

  async function encryptCandidateCdk(candidate: Readonly<SettingsCandidate>): Promise<string> {
    if (!cdkPlaintextKnown) {
      if (!candidate.hasCdkIntentWhileUnknown) {
        return lastPersisted.mirrorchyanCdkEncrypted;
      }
      if (!candidate.settings.mirrorchyanCdk) {
        return '';
      }
      return await persistence.encryptCdk(candidate.settings.mirrorchyanCdk);
    }

    if (!candidate.settings.mirrorchyanCdk) {
      return '';
    }
    if (candidate.settings.mirrorchyanCdk === rawEffective.mirrorchyanCdk) {
      return lastPersisted.mirrorchyanCdkEncrypted;
    }
    return await persistence.encryptCdk(candidate.settings.mirrorchyanCdk);
  }

  async function initializeSettings(): Promise<void> {
    if (initialization) {
      return await initialization;
    }

    initialization = initialize();
    return await initialization;
  }

  async function initialize(): Promise<void> {
    replaceStatus(rawStatus, { kind: 'loading' });
    try {
      const loaded = await persistence.load();
      let mirrorchyanCdk = '';
      if (loaded.mirrorchyanCdkEncrypted) {
        try {
          mirrorchyanCdk = await persistence.decryptCdk(loaded.mirrorchyanCdkEncrypted);
          cdkPlaintextKnown = true;
          cdkDecryptionError = undefined;
          hasCdkIntentWhileUnknown = false;
        } catch (error) {
          cdkPlaintextKnown = false;
          cdkDecryptionError = error;
        }
      } else {
        cdkPlaintextKnown = true;
        cdkDecryptionError = undefined;
        hasCdkIntentWhileUnknown = false;
      }
      const loadedDraft = settingsDraftFromPersisted(loaded, mirrorchyanCdk);
      replaceSettings(rawDraft, loadedDraft);
      replaceSettings(rawDraft, initializationEdits);
      replaceSettings(rawEffective, loadedDraft);
      lastPersisted = clonePersisted(loaded);
      failedRevision = undefined;
      initialized = true;
      replaceStatus(rawStatus, readyStatus());
      ensureWriter();
    } catch (error) {
      const fallback = createDefaultSettingsDraft();
      replaceSettings(rawDraft, fallback);
      replaceSettings(rawDraft, initializationEdits);
      replaceSettings(rawEffective, fallback);
      lastPersisted = clonePersisted(DEFAULT_OEA_CONFIG);
      cdkPlaintextKnown = true;
      cdkDecryptionError = undefined;
      initialized = true;
      failedRevision = undefined;
      replaceStatus(rawStatus, { kind: 'load-error', error });
      ensureWriter();
    }
  }

  function retrySettingsSave(): void {
    if (!initialized) {
      return;
    }
    failedRevision = undefined;
    ensureWriter();
  }

  function discardSettingsDraft(): void {
    if (!initialized) {
      return;
    }
    replaceSettings(rawDraft, rawEffective);
    draftRevision += 1;
    failedRevision = undefined;
    if (!cdkPlaintextKnown) {
      hasCdkIntentWhileUnknown = false;
    }
    if (!writerRunning) {
      savedRevision = draftRevision;
      replaceStatus(rawStatus, readyStatus());
      return;
    }
    ensureWriter();
  }

  return {
    settingsDraft,
    effectiveSettings: readonly(rawEffective) as DeepReadonly<SettingsSnapshot>,
    settingsStatus: readonly(rawStatus) as DeepReadonly<SettingsStatus>,
    initializeSettings,
    retrySettingsSave,
    discardSettingsDraft,
  };
}

function createDraftProxy(
  rawDraft: SettingsDraft,
  edit: <Key extends keyof SettingsDraft>(key: Key, value: SettingsDraft[Key]) => void,
): SettingsDraft {
  return new Proxy({} as SettingsDraft, {
    get(_target: SettingsDraft, key: string | symbol): unknown {
      return rawDraft[key as keyof SettingsDraft];
    },
    set(_target: SettingsDraft, key: string | symbol, value: unknown): boolean {
      if (typeof key === 'string' && key in rawDraft) {
        edit(key as keyof SettingsDraft, value as never);
      }
      return true;
    },
    ownKeys(): ArrayLike<string | symbol> {
      return Reflect.ownKeys(rawDraft);
    },
    getOwnPropertyDescriptor(
      _target: SettingsDraft,
      key: string | symbol,
    ): PropertyDescriptor | undefined {
      if (key in rawDraft) {
        return { configurable: true, enumerable: true };
      }
      return undefined;
    },
  });
}

function normalizeSetting<Key extends keyof SettingsDraft>(
  key: Key,
  value: unknown,
): SettingsDraft[Key] | undefined {
  switch (key) {
    case 'soundVolume':
      return typeof value === 'number' && Number.isFinite(value)
        ? (Math.min(1, Math.max(0, value)) as SettingsDraft[Key])
        : undefined;
    case 'minimizeToTray':
    case 'autoDownloadUpdates':
    case 'autoInstallUpdates':
    case 'scanGuideEnabled':
      return (typeof value === 'boolean' ? value : undefined) as SettingsDraft[Key] | undefined;
    case 'updateSource':
      return (Object.values(UpdateSource).includes(value as UpdateSource) ? value : undefined) as
        SettingsDraft[Key] | undefined;
    case 'updateProxyMode':
      return (
        Object.values(UpdateProxyMode).includes(value as UpdateProxyMode) ? value : undefined
      ) as SettingsDraft[Key] | undefined;
    case 'mirrorchyanCdk':
    case 'updateProxyUrl':
      return (typeof value === 'string' ? value : undefined) as SettingsDraft[Key] | undefined;
  }
}

function createCandidate(
  rawDraft: SettingsDraft,
  hasCdkIntentWhileUnknown: boolean,
): Readonly<SettingsCandidate> {
  return Object.freeze({
    settings: Object.freeze({
      ...rawDraft,
      mirrorchyanCdk: rawDraft.mirrorchyanCdk.trim(),
    }),
    hasCdkIntentWhileUnknown,
  });
}

function replaceSettings(target: SettingsDraft, source: Readonly<Partial<SettingsDraft>>): void {
  Object.assign(target, source);
}

function replaceStatus(target: MutableSettingsStatus, source: SettingsStatus): void {
  for (const key of Object.keys(target) as Array<keyof MutableSettingsStatus>) {
    delete target[key];
  }
  Object.assign(target, source);
}

function clonePersisted(config: Readonly<PersistedOeaConfig>): PersistedOeaConfig {
  return { ...config };
}
