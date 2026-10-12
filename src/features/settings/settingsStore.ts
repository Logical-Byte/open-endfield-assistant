import type { Settings } from '@/features/settings/types/settings';
import { readonly, shallowRef, type Ref } from 'vue';

export type DraftSettings = Omit<
  Settings,
  'majorVersion' | 'minorVersion' | 'mirrorchyanCdkEncrypted' | 'scanTipsDismissedVersion'
> & {
  /** 可编辑的 Mirror 酱 CDK 明文。`null` 表示解密失败。 */
  mirrorchyanCdk: string | null;
  /** 是否展示档案扫描启动提示。 */
  scanGuideEnabled: boolean;
};

export type SettingsState =
  | { status: 'initializing' }
  | {
      status: 'unavailable';
      reason: { type: 'initialize-error'; error: Error } | { type: 'unsupported' };
    }
  | {
      status: 'ready';
      draft: Readonly<DraftSettings>;
      effective: Readonly<DraftSettings>;
      saveError: Error | null;
    };

// 修改 ScanGuide 文案且需要所有用户重新确认时递增。无需改变设置文件版本。
const CURRENT_SCAN_TIPS_VERSION = 1;

/** 设置 store 使用的持久化边界，生产环境连接 Tauri IPC，测试中可替换为受控 Promise。 */
interface SettingsPersistence {
  load(): Promise<Settings>;
  save(settings: Settings): Promise<void>;
  encrypt(plain: string): Promise<string>;
  decrypt(encrypted: string): Promise<string>;
}

/** 设置 store 对调用方公开的只读状态和写入操作。 */
interface SettingsStore {
  state: Readonly<Ref<SettingsState>>;
  initialize(): Promise<void>;
  markUnsupported(): void;
  edit(patch: Partial<DraftSettings>): void;
  retry(): void;
}

/** 创建设置 store。`io` 可替换，使测试能直接控制初始化、加密和保存的完成时机。 */
export function createSettingsStore(io: SettingsPersistence): SettingsStore {
  const state = shallowRef<SettingsState>({ status: 'initializing' });
  // 最近一次成功初始化或保存的完整 DTO，用来保留版本字段和可复用的 CDK 密文。
  let persisted: Settings | null = null;
  // 同一个初始化 Promise 返回给并发调用方。失败后清空以允许重试。
  let initialization: Promise<void> | null = null;
  // 防止多个 write() 并发执行，使 io.encrypt() 和 io.save() 严格串行。
  let saving = false;
  // pending 表示 ready.draft 内的数据等待保存，write() 应当处理。
  let pending = false;

  /** 首次从后端读取完整设置。成功进入 ready 后，后续调用直接返回。 */
  function initialize(): Promise<void> {
    if (state.value.status === 'ready') return Promise.resolve();
    if (initialization !== null) return initialization;

    state.value = { status: 'initializing' };
    initialization = initializeFromBackend().finally(() => {
      initialization = null;
    });
    return initialization;
  }

  async function initializeFromBackend(): Promise<void> {
    try {
      const settings = await io.load();
      let cdk: string | null = '';
      if (settings.mirrorchyanCdkEncrypted) {
        try {
          cdk = await io.decrypt(settings.mirrorchyanCdkEncrypted);
        } catch {
          cdk = null;
        }
      }
      persisted = { ...settings };
      const draft = toDraft(settings, cdk);
      state.value = {
        status: 'ready',
        draft,
        effective: draft,
        saveError: null,
      };
    } catch (error) {
      state.value = {
        status: 'unavailable',
        reason: { type: 'initialize-error', error: asError(error) },
      };
    }
  }

  /** 纯浏览器模式没有设置后端，以 unavailable 状态明确表达能力边界。 */
  function markUnsupported(): void {
    if (state.value.status === 'ready') return;
    state.value = { status: 'unavailable', reason: { type: 'unsupported' } };
  }

  /** 将 patch 合并到 draft，并安排串行保存。首次初始化完成前忽略编辑。 */
  function edit(patch: Partial<DraftSettings>): void {
    const current = state.value;
    if (current.status !== 'ready') return;

    const next = { ...current.draft, ...patch };
    if (patch.soundVolume !== undefined) {
      // 非有限音量保留原值，同次编辑的其他字段仍可提交。
      next.soundVolume = Number.isFinite(patch.soundVolume)
        ? Math.min(1, Math.max(0, patch.soundVolume))
        : current.draft.soundVolume;
    }
    if (typeof patch.mirrorchyanCdk === 'string') {
      next.mirrorchyanCdk = patch.mirrorchyanCdk.trim();
    }
    if (
      current.saveError === null &&
      Object.keys(next).every(
        (key) => next[key as keyof DraftSettings] === current.draft[key as keyof DraftSettings],
      )
    ) {
      return;
    }

    // saveError 存在时，相同值也会重新提交，使失败后的下一次编辑可以恢复保存。
    state.value = { ...current, draft: next };
    pending = true;
    void write();
  }

  /** 保存失败后重新提交完整 draft。 */
  function retry(): void {
    const current = state.value;
    if (current.status !== 'ready' || current.saveError === null) return;
    pending = true;
    void write();
  }

  /** 循环处理 pending，每轮捕获一个 draft，保存成功后才发布为 effective。 */
  async function write(): Promise<void> {
    if (saving) return;
    const initial = state.value;
    if (initial.status !== 'ready' || persisted === null) return;

    saving = true;
    state.value = { ...initial, saveError: null };
    try {
      while (pending) {
        pending = false;
        const current: SettingsState = state.value;
        if (current.status !== 'ready' || persisted === null) return;

        // edit() 替换 draft，因此 await 期间再次编辑不会修改本轮 candidate。
        const candidate: Readonly<DraftSettings> = current.draft;
        const effective = current.effective;
        const { mirrorchyanCdk, scanGuideEnabled, ...values } = candidate;
        try {
          // 默认复用最近一次成功保存的密文，只有已知明文发生变化时才重新加密。
          let encrypted = persisted.mirrorchyanCdkEncrypted;
          if (mirrorchyanCdk !== null && mirrorchyanCdk !== effective.mirrorchyanCdk) {
            encrypted = mirrorchyanCdk ? await io.encrypt(mirrorchyanCdk) : '';
          }
          const settings: Settings = {
            ...persisted,
            ...values,
            mirrorchyanCdkEncrypted: encrypted,
            // 仅显式切换提示时编码版本，其他编辑保留磁盘中的历史或更高版本号。
            scanTipsDismissedVersion:
              scanGuideEnabled === effective.scanGuideEnabled
                ? persisted.scanTipsDismissedVersion
                : scanGuideEnabled
                  ? 0
                  : CURRENT_SCAN_TIPS_VERSION,
          };
          await io.save(settings);
          persisted = settings;
          const latest: SettingsState = state.value;
          if (latest.status === 'ready') {
            state.value = { ...latest, effective: candidate };
          }
        } catch (error) {
          // 有更新的 draft 等待处理时继续循环，否则保留错误供用户重试。
          if (!pending) {
            const latest: SettingsState = state.value;
            if (latest.status === 'ready') {
              state.value = { ...latest, saveError: asError(error) };
            }
          }
        }
      }
    } finally {
      saving = false;
    }
  }

  return {
    state: readonly(state),
    initialize,
    markUnsupported,
    edit,
    retry,
  };
}

function toDraft(settings: Settings, mirrorchyanCdk: string | null): DraftSettings {
  return {
    uiLocale: settings.uiLocale,
    minimizeToTray: settings.minimizeToTray,
    soundVolume: settings.soundVolume,
    updateSource: settings.updateSource,
    updateProxyMode: settings.updateProxyMode,
    updateProxyUrl: settings.updateProxyUrl,
    autoDownloadUpdates: settings.autoDownloadUpdates,
    autoInstallUpdates: settings.autoInstallUpdates,
    scanGuideEnabled: settings.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION,
    mirrorchyanCdk,
  };
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
