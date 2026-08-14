import { OeaConfig, UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import { cdkDecrypt, cdkEncrypt, loadOeaConfig, logError, saveOeaConfig } from '@/utils/tauri';
import { ref, watch } from 'vue';

/** 更新源选项 */
export const updateSourceItems = [
  { label: 'Mirror酱', value: UpdateSource.Mirrorchyan },
  { label: 'GitHub', value: UpdateSource.Github },
];

/** 更新代理模式选项 */
export const proxyModeItems = [
  { label: '不使用代理', value: UpdateProxyMode.None },
  { label: '系统代理', value: UpdateProxyMode.System },
  { label: '自定义代理', value: UpdateProxyMode.Custom },
];

export const CURRENT_MAJOR_VERSION: number = 0 as const;
export const CURRENT_MINOR_VERSION: number = 1 as const;

/**
 * 当前档案扫描提示的版本号（与 `ScanGuide.vue` 中的提示文案同源）。
 *
 * 展示规则：`oeaConfig.scanTipsDismissedVersion < CURRENT_SCAN_TIPS_VERSION` 时展示提示；
 * 用户勾选「下次更新前不再提示」并点击「我知道了」后，`scanTipsDismissedVersion`
 * 会被写入本值并持久化到后端配置（`config/oea_config.json`），之后本版本内不再展示。
 *
 * ▍让所有用户（含已确认过的）重新看一次新版提示：
 *   修改 `ScanGuide.vue` 的提示文案，同时把本常量递增 1（例如 1 → 2）。
 *   老用户已确认的版本号（1）将小于新版本（2），下次启动会重新看到新提示；
 *   从未确认过的新用户（0 < 2）同样会看到。
 *   这只改了前端常量与文案，不涉及 config 结构，因此无需 bump `minorVersion`。
 *
 * ▍更新提示文案但无需老用户重看：
 *   只修改 `ScanGuide.vue` 的提示文案，保持本常量不变。已确认过的用户不会重新看到；
 *   只有从未确认过的新用户会看到最新文案。
 */
export const CURRENT_SCAN_TIPS_VERSION: number = 1 as const;

export const DEFAULT_OEA_CONFIG: OeaConfig = {
  majorVersion: CURRENT_MAJOR_VERSION,
  minorVersion: CURRENT_MINOR_VERSION,
  minimizeToTray: false,
  soundVolume: 0.5,
  updateSource: UpdateSource.Mirrorchyan,
  mirrorchyanCdkEncrypted: '',
  updateProxyMode: UpdateProxyMode.System,
  updateProxyUrl: '',
  autoDownloadUpdates: true,
  autoInstallUpdates: true,
  scanTipsDismissedVersion: 0,
} as const;

export const oeaConfig = ref<OeaConfig>(DEFAULT_OEA_CONFIG);
/** 配置是否已从磁盘加载完成。加载完成前 UI 不应依据默认值渲染（如扫描提示），避免启动时短暂闪烁。 */
export const configLoaded = ref(false);
/** Mirror酱 CDK 明文（仅内存共享，不落盘；磁盘只存 `oeaConfig.mirrorchyanCdkEncrypted` 密文）。 */
export const mirrorchyanCdk = ref('');
export const saving = ref(false);

/**
 * 深拷贝配置。
 *
 * ref 会把对象值包装为响应式代理（Proxy），structuredClone 无法克隆 Proxy；
 * 配置为纯 JSON 数据，故用 JSON 序列化实现深拷贝。
 */
function deepClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value));
}

/** 明文变化后加密并写入配置密文（由配置深监听统一落盘）。 */
async function saveMirrorchyanCdk(plain: string): Promise<void> {
  if (!plain) {
    oeaConfig.value.mirrorchyanCdkEncrypted = '';
    return;
  }
  try {
    oeaConfig.value.mirrorchyanCdkEncrypted = await cdkEncrypt(plain);
  } catch (error) {
    useToast().add({
      title: '保存 CDK 失败',
      description: error instanceof Error ? error.message : String(error),
      icon: 'i-lucide-triangle-alert',
      color: 'error',
    });
  }
}

/** 最近一次成功保存到磁盘的配置快照（保存失败时回滚到此值）。 */
let lastSaved: OeaConfig = deepClone(DEFAULT_OEA_CONFIG);
/** 回滚进行中标志：拦截回滚赋值触发的重复保存。 */
let restoring = false;

export async function initOeaConfig() {
  const toast = useToast();

  // 加载配置，失败时使用默认配置。
  try {
    oeaConfig.value = await loadOeaConfig();
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logError(`加载配置失败，使用默认配置: ${errorMessage}`);
    oeaConfig.value = deepClone(DEFAULT_OEA_CONFIG);
  }
  // 保存快照，回滚时使用。
  lastSaved = deepClone(oeaConfig.value);
  // 标记配置已就绪：此后 UI 才可依据真实配置渲染（如扫描提示），避免用默认值短暂闪现。
  configLoaded.value = true;

  // 解密密文填充内存明文（必须在注册明文 watch 之前，避免初始化即触发一次加密写回）。
  if (oeaConfig.value.mirrorchyanCdkEncrypted) {
    try {
      mirrorchyanCdk.value = await cdkDecrypt(oeaConfig.value.mirrorchyanCdkEncrypted);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      logError(`解密 CDK 失败，视为未设置: ${errorMessage}`);
    }
  }

  // 明文变化 → 加密 → 写入配置密文（配置深监听负责落盘）。
  watch(mirrorchyanCdk, async (value: string) => {
    await saveMirrorchyanCdk(value.trim());
  });

  // 注意：deep watch 下新/旧值都是同一对象引用（深层修改不改变引用），
  // 无法用旧值回滚，因此单独维护 lastSaved 快照。
  // flush: 'sync' 保证回滚赋值立即命中 restoring 标志，避免回滚再次触发保存。
  watch(
    oeaConfig,
    async () => {
      if (restoring) {
        return;
      }

      saving.value = true;
      try {
        await saveOeaConfig(oeaConfig.value);
        lastSaved = deepClone(oeaConfig.value);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        logError(`保存配置失败，已回滚: ${errorMessage}`);
        restoring = true;
        oeaConfig.value = deepClone(lastSaved);
        restoring = false;
        toast.add({
          title: '保存配置失败',
          description: error instanceof Error ? error.message : String(error),
          icon: 'i-lucide-triangle-alert',
          color: 'error',
        });
      } finally {
        saving.value = false;
      }
    },
    { deep: true, flush: 'sync' },
  );
}
