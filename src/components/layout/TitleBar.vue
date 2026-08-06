<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onMounted, onUnmounted, ref } from 'vue';

// 仅 Windows 渲染自定义标题栏：Rust 端只在 Windows 移除系统标题栏，
// macOS/Linux 保留原生标题栏，此处不渲染，避免出现双标题栏。
const isWindows = navigator.userAgent.toLowerCase().includes('win');
const isTauri = '__TAURI_INTERNALS__' in window;

const appWindow = isTauri ? getCurrentWindow() : null;
const isMaximized = ref(false);

let unlistenResize: (() => void) | null = null;

async function refreshMaximized(): Promise<void> {
  if (!appWindow) return;
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch {
    // 非 Tauri 环境，忽略
  }
}

onMounted(async () => {
  if (!appWindow) return;
  try {
    await refreshMaximized();
    unlistenResize = await appWindow.onResized(() => {
      void refreshMaximized();
    });
  } catch {
    // 忽略
  }
});

onUnmounted(() => {
  unlistenResize?.();
});

function handleMinimize(): void {
  void appWindow?.minimize();
}

function handleToggleMaximize(): void {
  void appWindow?.toggleMaximize();
}

function handleClose(): void {
  void appWindow?.close();
}
</script>

<template>
  <div
    v-if="isTauri && isWindows"
    class="flex h-9 shrink-0 select-none items-center justify-between bg-default"
    data-tauri-drag-region
  >
    <div class="flex h-full items-center px-3" data-tauri-drag-region>
      <span class="text-sm font-semibold text-toned">OEA</span>
    </div>

    <div class="flex h-full">
      <button
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-muted hover:text-toned"
        title="最小化"
        type="button"
        @click="handleMinimize"
      >
        <UIcon class="size-4" name="i-lucide-minus" />
      </button>
      <button
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-muted hover:text-toned"
        :title="isMaximized ? '还原' : '最大化'"
        type="button"
        @click="handleToggleMaximize"
      >
        <UIcon v-if="isMaximized" class="size-3.5 rotate-180" name="i-lucide-copy" />
        <UIcon v-else class="size-3.5" name="i-lucide-square" />
      </button>
      <button
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-red-500 hover:text-white"
        title="关闭"
        type="button"
        @click="handleClose"
      >
        <UIcon class="size-4" name="i-lucide-x" />
      </button>
    </div>
  </div>
</template>
