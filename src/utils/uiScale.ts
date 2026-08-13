import { getWebviewZoom, logError, onWebviewZoomChanged } from '@/utils/tauri';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { watchDebounced } from '@vueuse/core';
import { ref } from 'vue';

/**
 * 当前 UI 缩放值（WebView2 `ZoomFactor` 的内存镜像）。
 * 缩放的唯一持久化由 WebView2 自身负责（写入用户数据目录），前端不再额外存储。
 */
export const uiScale = ref(1);

/** 从 WebView2 读取当前缩放因子，初始化 `uiScale`。 */
export async function initUiScale(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    const factor = await getWebviewZoom();
    if (Number.isFinite(factor)) {
      uiScale.value = factor;
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    logError(`读取 UI 缩放失败: ${errorMessage}`);
  }
}

/**
 * 将当前缩放值应用到 WebView2 窗口（`ZoomFactor`）。
 * 非 Tauri 环境或无法解析为有限数字时直接跳过。
 */
export async function applyUiScale(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const factor = Number(uiScale.value);
  if (!Number.isFinite(factor)) {
    return;
  }
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

if (isTauri()) {
  // 监听 WebView2 原生缩放（`Ctrl+滚轮` / `Ctrl+加减`）变化，同步回 `uiScale`。
  onWebviewZoomChanged((zoom) => {
    const factor = Number(zoom);
    if (Number.isFinite(factor)) {
      uiScale.value = factor;
    }
  });
}
