import type { OeaConfig } from '@/types/oeaConfig';
import { readonly, ref, shallowRef, type Ref } from 'vue';

/** 可编辑设置；null 表示 CDK 解密失败，保存其他字段时必须保留原密文。 */
export type SettingsDraft = Omit<
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
  draft: Readonly<Ref<Readonly<SettingsDraft>>>;
  effective: Readonly<Ref<Readonly<SettingsDraft>>>;
  loaded: Readonly<Ref<boolean>>;
  loading: Readonly<Ref<boolean>>;
  saving: Readonly<Ref<boolean>>;
  loadError: Readonly<Ref<Error | null>>;
  saveError: Readonly<Ref<Error | null>>;
  initialize(): Promise<void>;
  edit(patch: Partial<SettingsDraft>): void;
  retry(): void;
}

/** 设置保存的唯一写入入口。依赖可替换，测试能直接控制加密和保存的完成时机。 */
export function createConfigStore(defaults: OeaConfig, io: ConfigPersistence): ConfigStore {
  const draft = shallowRef(toDraft(defaults, ''));
  const effective = shallowRef(draft.value);
  const loaded = ref(false);
  const loading = ref(false);
  const saving = ref(false);
  const loadError = shallowRef<Error | null>(null);
  const saveError = shallowRef<Error | null>(null);
  let persisted = { ...defaults };
  let pending = false;

  async function initialize(): Promise<void> {
    if (loading.value || loaded.value) return;
    loading.value = true;
    loadError.value = null;
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
      loaded.value = true;
    } catch (error) {
      // IPC 加载失败时保持禁用，不能用前端占位默认值覆盖后端配置。
      loadError.value = asError(error);
    } finally {
      loading.value = false;
    }
  }

  function edit(patch: Partial<SettingsDraft>): void {
    if (!loaded.value) return;
    const next = { ...draft.value, ...patch };
    if (next.mirrorchyanCdk !== null) next.mirrorchyanCdk = next.mirrorchyanCdk.trim();
    if (
      !saveError.value &&
      Object.keys(next).every(
        (key) => next[key as keyof SettingsDraft] === draft.value[key as keyof SettingsDraft],
      )
    )
      return;
    draft.value = next;
    pending = true;
    void write();
  }

  function retry(): void {
    if (!saveError.value) return;
    pending = true;
    void write();
  }

  async function write(): Promise<void> {
    if (saving.value) return;
    saving.value = true;
    saveError.value = null;
    try {
      while (pending) {
        pending = false;
        // 每次 edit 都替换对象；await 期间的编辑不会修改已捕获的候选。
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
          // 新编辑会合并成下一份候选；最新候选失败后停止，等待用户重试或继续编辑。
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
    loaded: readonly(loaded),
    loading: readonly(loading),
    saving: readonly(saving),
    loadError: readonly(loadError),
    saveError: readonly(saveError),
    initialize,
    edit,
    retry,
  };
}

function toDraft(config: OeaConfig, mirrorchyanCdk: string | null): SettingsDraft {
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
