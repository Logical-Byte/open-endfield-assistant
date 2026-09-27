import type { OeaConfig } from '@/types/oeaConfig';
import { readonly, ref, shallowRef, type Ref } from 'vue';

/** 可编辑设置。`mirrorchyanCdk === null` 表示解密失败，保存其他字段时保留原密文。 */
export type DraftSettings = Omit<
  OeaConfig,
  'majorVersion' | 'minorVersion' | 'mirrorchyanCdkEncrypted' | 'scanTipsDismissedVersion'
> & { mirrorchyanCdk: string | null; scanGuideEnabled: boolean };

// 修改 ScanGuide 文案且需要所有用户重新确认时递增；无需改变配置文件版本。
const CURRENT_SCAN_TIPS_VERSION = 1;

/** 配置 store 使用的持久化边界，生产环境连接 Tauri IPC，测试中可替换为受控 Promise。 */
interface ConfigPersistence {
  load(): Promise<OeaConfig>;
  save(config: OeaConfig): Promise<void>;
  encrypt(plain: string): Promise<string>;
  decrypt(encrypted: string): Promise<string>;
}

/** 配置 store 对调用方公开的只读状态和写入操作。 */
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

/** 创建设置 store。`io` 可替换，使测试能直接控制加载、加密和保存的完成时机。 */
export function createConfigStore(defaults: OeaConfig, io: ConfigPersistence): ConfigStore {
  // `draft.value` 始终通过替换整个对象更新，使 `write()` 捕获的对象在 `await` 期间保持不变。
  const draft = shallowRef(toDraft(defaults, ''));
  // 最近一次成功加载或保存的 `draft` 快照，业务流程只能依据这份设置作出决定。
  const effective = shallowRef(draft.value);
  // 成功加载配置并完成 CDK 解密尝试后设为 `true`，控制持久化设置是否可以编辑。
  const initialized = ref(false);
  // 阻止多个 `initialize()` 同时读取配置，初始化失败后会恢复为 `false` 以允许重试。
  const initializing = ref(false);
  // 防止多个 `write()` 并发执行，使 `io.encrypt()` 和 `io.save()` 严格串行。
  const saving = ref(false);
  // 只记录加载配置的错误，CDK 解密失败由 `draft.mirrorchyanCdk === null` 表示。
  const initializeError = shallowRef<Error | null>(null);
  // 记录最新 `draft` 对应候选的加密或保存错误，再次开始 `write()` 时清除。
  const saveError = shallowRef<Error | null>(null);
  // 最近一次成功加载或保存的完整 DTO，用来保留版本字段和可复用的 CDK 密文。
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
    // `saveError.value` 存在时，相同值也会重新提交，使失败后的下一次编辑可以恢复保存。
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
        const { mirrorchyanCdk, scanGuideEnabled, ...values } = candidate;
        try {
          // 默认复用最近一次成功保存的密文，只有已知明文发生变化时才重新加密。
          let encrypted = persisted.mirrorchyanCdkEncrypted;
          if (mirrorchyanCdk !== null && mirrorchyanCdk !== effective.value.mirrorchyanCdk) {
            encrypted = mirrorchyanCdk ? await io.encrypt(mirrorchyanCdk) : '';
          }
          const config = {
            ...persisted,
            ...values,
            mirrorchyanCdkEncrypted: encrypted,
            // 仅显式切换提示时编码版本，其他编辑保留磁盘中的历史或更高版本号。
            scanTipsDismissedVersion:
              scanGuideEnabled === effective.value.scanGuideEnabled
                ? persisted.scanTipsDismissedVersion
                : scanGuideEnabled
                  ? 0
                  : CURRENT_SCAN_TIPS_VERSION,
          };
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
    scanGuideEnabled: config.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION,
    mirrorchyanCdk,
  };
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
