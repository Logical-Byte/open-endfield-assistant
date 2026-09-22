import { getWebviewZoom, logError, onWebviewZoomChanged } from '@/utils/tauri';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { watchDebounced } from '@vueuse/core';
import { customRef, type Ref } from 'vue';

const MIN_UI_SCALE = 0.5;
const MAX_UI_SCALE = 2;

/**
 * 归一化 WebView 缩放值。
 *
 * 缩放仍是独立于 OEA 配置 DTO 的 WebView 状态；此处只保证滑块与原生事件不会把
 * 非法比例传播到窗口。
 */
function normalizeUiScale(value: unknown): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return undefined;
  }
  return Math.min(MAX_UI_SCALE, Math.max(MIN_UI_SCALE, value));
}

/**
 * 当前 UI 缩放值（WebView2 `ZoomFactor` 的内存镜像）。
 * 缩放的唯一持久化由 WebView2 自身负责（写入用户数据目录），前端不再额外存储。
 */
export const uiScale: Ref<number> = createUiScaleRef();

function createUiScaleRef(): Ref<number> {
  let current = 1;
  return customRef<number>((track: () => void, trigger: () => void) => ({
    get(): number {
      track();
      return current;
    },
    set(value: number): void {
      const normalized = normalizeUiScale(value);
      if (normalized === undefined || normalized === current) {
        return;
      }
      current = normalized;
      trigger();
    },
  }));
}

/** 从 WebView2 读取当前缩放因子，初始化 `uiScale`。 */
export async function initUiScale(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    const factor = await getWebviewZoom();
    uiScale.value = factor;
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logError(`读取 UI 缩放失败: ${errorMessage}`);
  }

  if (isTauri()) {
    // 监听 WebView2 原生缩放（`Ctrl+滚轮` / `Ctrl+加减`）变化，同步回 `uiScale`。
    await onWebviewZoomChanged((zoom) => {
      uiScale.value = zoom;
    });
  }
}

/**
 * 将当前缩放值应用到 WebView2 窗口（`ZoomFactor`）。
 * 非 Tauri 环境时直接跳过。`uiScale` 在写入时已经拒绝非有限值并限制范围。
 */
export async function applyUiScale(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const factor = uiScale.value;
  try {
    return await getCurrentWebview().setZoom(factor);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logError(`应用 UI 缩放失败: ${errorMessage}`);
  }
}

// 滑块拖动时防抖应用到窗口。
watchDebounced(
  uiScale,
  async () => {
    await applyUiScale();
  },
  { debounce: 750 },
);
