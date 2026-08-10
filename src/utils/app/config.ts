import type { OeaConfig } from '@/types/oeaConfig';
import { loadOeaConfig, saveOeaConfig } from '@/utils/tauri';
import { ref, watch } from 'vue';

export const VERSION: [number, number] = [0, 0];

export const DEFAULT_OEA_CONFIG: OeaConfig = {
  version: VERSION,
  minimizeToTray: false,
  soundVolume: 0.8,
};

export const oeaConfig = ref<OeaConfig>(DEFAULT_OEA_CONFIG);
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

/** 最近一次成功保存到磁盘的配置快照（保存失败时回滚到此值）。 */
let lastSaved: OeaConfig = deepClone(DEFAULT_OEA_CONFIG);
/** 回滚进行中标志：拦截回滚赋值触发的重复保存。 */
let restoring = false;

let initialized = false;

export async function initOeaConfig() {
  if (initialized) {
    return;
  }
  initialized = true;

  const toast = useToast();

  try {
    oeaConfig.value = await loadOeaConfig();
  } catch (error) {
    console.error('加载配置失败，使用默认配置', error);
    oeaConfig.value = deepClone(DEFAULT_OEA_CONFIG);
  }
  lastSaved = deepClone(oeaConfig.value);

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
        console.error('保存配置失败，已回滚', error);
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
