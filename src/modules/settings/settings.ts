import { reactive, readonly, type DeepReadonly } from 'vue';

import { UpdateProxyMode, UpdateSource, type OeaConfig } from '@/types/oeaConfig';

import type { SettingsDraft, SettingsSnapshot, SettingsStatus } from './model';
import {
  createDefaultSettingsDraft,
  DEFAULT_OEA_CONFIG,
  persistedFromSettingsDraft,
  settingsDraftFromPersisted,
  type SettingsPersistence,
} from './persistence';

export interface SettingsModule {
  settingsDraft: SettingsDraft;
  effectiveSettings: DeepReadonly<SettingsSnapshot>;
  settingsStatus: DeepReadonly<SettingsStatus>;
  initializeSettings(): Promise<void>;
  retrySettingsSave(): void;
  discardSettingsDraft(): void;
}

type MutableSettingsStatus = {
  kind: SettingsStatus['kind'];
  error?: unknown;
};

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
  let lastPersisted: OeaConfig = clonePersisted(DEFAULT_OEA_CONFIG);

  const settingsDraft = createDraftProxy(rawDraft, edit);

  function edit<Key extends keyof SettingsDraft>(key: Key, value: SettingsDraft[Key]): void {
    const normalized = normalizeSetting(key, value);
    if (normalized === undefined || rawDraft[key] === normalized) {
      return;
    }

    rawDraft[key] = normalized as SettingsDraft[Key];
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

  async function runWriter(): Promise<void> {
    try {
      while (hasPendingWork()) {
        const targetRevision = draftRevision;
        const candidate = createCandidate(rawDraft);
        replaceStatus(rawStatus, { kind: 'saving' });

        try {
          const encryptedCdk = await encryptCandidateCdk(candidate);
          const persistedCandidate = Object.freeze(
            persistedFromSettingsDraft(candidate, lastPersisted, encryptedCdk),
          );
          await persistence.save(persistedCandidate);

          lastPersisted = clonePersisted(persistedCandidate);
          replaceSettings(rawEffective, candidate);
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

      replaceStatus(rawStatus, { kind: 'idle' });
    } finally {
      writerRunning = false;
      // edit 与 finally 之间可以交错；重新检查避免遗漏最后一次唤醒。
      ensureWriter();
    }
  }

  async function encryptCandidateCdk(candidate: Readonly<SettingsDraft>): Promise<string> {
    if (!candidate.mirrorchyanCdk) {
      return '';
    }
    if (candidate.mirrorchyanCdk === rawEffective.mirrorchyanCdk) {
      return lastPersisted.mirrorchyanCdkEncrypted;
    }
    return await persistence.encryptCdk(candidate.mirrorchyanCdk);
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
      const mirrorchyanCdk = loaded.mirrorchyanCdkEncrypted
        ? await persistence.decryptCdk(loaded.mirrorchyanCdkEncrypted)
        : '';
      const loadedDraft = settingsDraftFromPersisted(loaded, mirrorchyanCdk);
      replaceSettings(rawDraft, loadedDraft);
      replaceSettings(rawEffective, loadedDraft);
      lastPersisted = clonePersisted(loaded);
      savedRevision = draftRevision;
      failedRevision = undefined;
      initialized = true;
      replaceStatus(rawStatus, { kind: 'idle' });
      ensureWriter();
    } catch (error) {
      const fallback = createDefaultSettingsDraft();
      replaceSettings(rawDraft, fallback);
      replaceSettings(rawEffective, fallback);
      lastPersisted = clonePersisted(DEFAULT_OEA_CONFIG);
      savedRevision = draftRevision;
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
    if (!writerRunning) {
      savedRevision = draftRevision;
      replaceStatus(rawStatus, { kind: 'idle' });
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

function createCandidate(rawDraft: SettingsDraft): Readonly<SettingsDraft> {
  return Object.freeze({
    ...rawDraft,
    mirrorchyanCdk: rawDraft.mirrorchyanCdk.trim(),
  });
}

function replaceSettings(target: SettingsDraft, source: Readonly<SettingsDraft>): void {
  Object.assign(target, source);
}

function replaceStatus(target: MutableSettingsStatus, source: SettingsStatus): void {
  for (const key of Object.keys(target) as Array<keyof MutableSettingsStatus>) {
    delete target[key];
  }
  Object.assign(target, source);
}

function clonePersisted(config: Readonly<OeaConfig>): OeaConfig {
  return { ...config };
}
