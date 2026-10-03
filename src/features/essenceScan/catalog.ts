import { readonly, ref, shallowRef, type DeepReadonly, type Ref } from 'vue';
import { getEssenceCatalog } from './ipc';
import type { EssenceCatalog } from './types';

const data = shallowRef<EssenceCatalog | null>(null);
const status = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
const error = ref<string | null>(null);
let pending: Promise<void> | null = null;

export const catalog: DeepReadonly<Ref<EssenceCatalog | null>> = readonly(data);
export const catalogStatus = readonly(status);
export const catalogError = readonly(error);

/** App 在桌面环境初始化一次，失败后可以从页面重试。 */
export async function initEssenceCatalog(): Promise<void> {
  if (status.value === 'ready') return;
  if (pending !== null) return await pending;
  status.value = 'loading';
  error.value = null;
  pending = loadCatalog();
  await pending;
}

async function loadCatalog(): Promise<void> {
  try {
    data.value = await getEssenceCatalog();
    status.value = 'ready';
  } catch (cause: unknown) {
    error.value = cause instanceof Error ? cause.message : String(cause);
    status.value = 'error';
  } finally {
    pending = null;
  }
}
