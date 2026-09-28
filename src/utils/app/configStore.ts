import type { OeaConfig } from '@/types/oeaConfig';
import { readonly, ref, shallowRef, type Ref } from 'vue';

/** 可编辑设置。`mirrorchyanCdk === null` 表示解密失败，保存其他字段时保留原密文。 */
export type DraftSettings = Omit<
  OeaConfig,
  'majorVersion' | 'minorVersion' | 'mirrorchyanCdkEncrypted'
> & { mirrorchyanCdk: string | null };

interface ConfigPersistence {
  load(): Promise<OeaConfig>;
  save(config: OeaConfig): Promise<void>;
  encrypt(plain: string): Promise<string>;
  decrypt(encrypted: string): Promise<string>;
}

interface ConfigStore {
  draft: Readonly<Ref<Readonly<DraftSettings>>>;
  effective: Readonly<Ref<Readonly<DraftSettings>>>;
  initialized: Readonly<Ref<boolean>>;
  initializing: Readonly<Ref<boolean>>;
  saving: Readonly<Ref<boolean>>;
  initializeError: Readonly<Ref<Error | null>>;
  saveError: Readonly<Ref<Error | null>>;
  initialize(): Promise<void>;
  edit(patch: Partial<DraftSettings>): void;
  retry(): void;
}

/** 设置保存的唯一写入入口。依赖可替换，测试能直接控制加密和保存的完成时机。 */
export function createConfigStore(defaults: OeaConfig, io: ConfigPersistence): ConfigStore {
  const draft = shallowRef(toDraft(defaults, ''));
  const effective = shallowRef(draft.value);
  const initialized = ref(false);
  const initializing = ref(false);
  const saving = ref(false);
  const initializeError = shallowRef<Error | null>(null);
  const saveError = shallowRef<Error | null>(null);
  let persisted = { ...defaults };
  /** pending 为 true 表示 draft 内的数据等待保存，write() 应当处理。*/
  let pending = false;

  /**
   * 通过 `io.load()` 和 `io.decrypt()` 初始化 `persisted`、`draft` 和 `effective`。
   * 初始化成功后将 `initialized.value` 设为 `true`，失败则写入 `initializeError` 并保持 `initialized.value === false`。
   */
  async function initialize(): Promise<void> {
    if (initializing.value || initialized.value) return;
    initializing.value = true;
    initializeError.value = null;
    try {
      const config = await io.load();
      let cdk: string | null = '';
      if (config.mirrorchyanCdkEncrypted) {
        try {
          cdk = await io.decrypt(config.mirrorchyanCdkEncrypted);
        } catch {
          cdk = null;
        }
      }
      persisted = { ...config };
      draft.value = toDraft(config, cdk);
      effective.value = draft.value;
      initialized.value = true;
    } catch (error) {
      // `initialized.value` 保持 `false`，阻止 `edit()` 将 `defaults` 写回后端。
      initializeError.value = asError(error);
    } finally {
      initializing.value = false;
    }
  }

  /**
   * 将 `patch` 合并到 `draft`，设置 `pending = true` 并调用 `write()`。
   * `initialized.value === false` 时忽略修改。`saving.value === true` 时由 `write()` 的下一轮读取最新 `draft`。
   */
  function edit(patch: Partial<DraftSettings>): void {
    if (!initialized.value) return;
    const next = { ...draft.value, ...patch };
    if (typeof patch.mirrorchyanCdk === 'string') {
      next.mirrorchyanCdk = patch.mirrorchyanCdk.trim();
    }
    if (
      !saveError.value &&
      Object.keys(next).every(
        (key) => next[key as keyof DraftSettings] === draft.value[key as keyof DraftSettings],
      )
    )
      return;
    draft.value = next;
    pending = true;
    void write();
  }

  /** `saveError.value` 非 `null` 时设置 `pending = true`，调用 `write()` 重新提交完整 `draft`。 */
  function retry(): void {
    if (!saveError.value) return;
    pending = true;
    void write();
  }

  /**
   * `saving.value === true` 时直接返回，避免并发调用 `io.encrypt()` 和 `io.save()`。
   * 循环处理 `pending`，每轮捕获 `candidate`，保存成功后赋给 `effective.value`。
   */
  async function write(): Promise<void> {
    if (saving.value) return;
    saving.value = true;
    saveError.value = null;
    try {
      while (pending) {
        pending = false;
        // `edit()` 替换 `draft.value`，因此 `await` 期间再次调用 `edit()` 不会修改本轮的 `candidate`。
        const candidate = draft.value;
        const { mirrorchyanCdk, ...values } = candidate;
        try {
          let encrypted = persisted.mirrorchyanCdkEncrypted;
          if (mirrorchyanCdk !== null && mirrorchyanCdk !== effective.value.mirrorchyanCdk) {
            encrypted = mirrorchyanCdk ? await io.encrypt(mirrorchyanCdk) : '';
          }
          const config = { ...persisted, ...values, mirrorchyanCdkEncrypted: encrypted };
          await io.save(config);
          persisted = config;
          effective.value = candidate;
        } catch (error) {
          // `pending === true` 时继续保存最新 `draft`，否则写入 `saveError` 并结束循环。
          if (!pending) saveError.value = asError(error);
        }
      }
    } finally {
      saving.value = false;
    }
  }

  return {
    draft: readonly(draft),
    effective: readonly(effective),
    initialized: readonly(initialized),
    initializing: readonly(initializing),
    saving: readonly(saving),
    initializeError: readonly(initializeError),
    saveError: readonly(saveError),
    initialize,
    edit,
    retry,
  };
}

function toDraft(config: OeaConfig, mirrorchyanCdk: string | null): DraftSettings {
  return {
    minimizeToTray: config.minimizeToTray,
    soundVolume: config.soundVolume,
    updateSource: config.updateSource,
    updateProxyMode: config.updateProxyMode,
    updateProxyUrl: config.updateProxyUrl,
    autoDownloadUpdates: config.autoDownloadUpdates,
    autoInstallUpdates: config.autoInstallUpdates,
    scanTipsDismissedVersion: config.scanTipsDismissedVersion,
    mirrorchyanCdk,
  };
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
