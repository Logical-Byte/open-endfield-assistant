<script setup lang="ts">
import { UpdateCheckStatus } from '@/types/update';
import { appVersion } from '@/utils/app/appVersion';
import { startUpdate, updateCheckResult } from '@/utils/app/update';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onMounted, onUnmounted, ref } from 'vue';

// 应用图标：使用 /favicon.ico（dev 由 vite 中间件提供，构建后位于 dist 根目录）。
// 用动态绑定避免 Vite 把它当作模块导入解析。
const faviconUrl = '/favicon.ico';

/** 检测到新版本时的炫彩提示文案（未检测到更新时为 `null`）。 */
const updateNotice = computed<string | null>(() => {
  if (updateCheckResult.value.status !== UpdateCheckStatus.HasUpdate) {
    return null;
  }
  const version = updateCheckResult.value.result.data?.version_name;
  return version ? `检测到新版本：v${version.replace(/^v/, '')}` : null;
});

const appWindow = isTauri() ? getCurrentWindow() : null;
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
</script>

<template>
  <div
    v-if="isTauri()"
    class="flex h-7.5 shrink-0 items-center justify-between border-b border-default bg-default select-none"
    data-tauri-drag-region
  >
    <div class="flex h-full items-center gap-1.5 px-3" data-tauri-drag-region>
      <img
        alt="OEA"
        class="pointer-events-none size-4 object-contain"
        draggable="false"
        :src="faviconUrl"
      />
      <span
        v-if="updateNotice"
        class="titlebar-update-notice text-xs font-bold"
        @click="startUpdate"
      >
        {{ updateNotice }}
      </span>
      <span class="pointer-events-none font-ui text-xs text-toned">
        OEA<span v-if="appVersion"> v{{ appVersion }}</span>
      </span>
    </div>

    <div class="flex h-full">
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-accented hover:text-toned"
        title="最小化"
        type="button"
        @click="appWindow?.minimize"
      >
        <WindowMinimizeIcon class="size-4" />
      </button>
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-accented hover:text-toned"
        :title="isMaximized ? '还原' : '最大化'"
        type="button"
        @click="appWindow?.toggleMaximize"
      >
        <WindowRestoreIcon v-if="isMaximized" class="size-4" />
        <WindowMaximizeIcon v-else class="size-4" />
      </button>
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-red-500 hover:text-white"
        title="关闭"
        type="button"
        @click="appWindow?.close"
      >
        <WindowCloseIcon class="size-4" />
      </button>
    </div>
  </div>
</template>

<style lang="css" scoped>
.titlebar-update-notice {
  background-image: linear-gradient(
    90deg,
    var(--color-red-500),
    /* var(--color-orange-500), */ var(--color-yellow-500),
    /* var(--color-lime-500), */ var(--color-green-500),
    /* var(--color-teal-500), */ var(--color-cyan-500),
    /* var(--color-sky-500), */ var(--color-blue-500),
    /* var(--color-violet-500), */ var(--color-fuchsia-500) /* var(--color-pink-500) */
  );
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  animation: titlebar-update-notice-flow 3s linear infinite;
}

@keyframes titlebar-update-notice-flow {
  to {
    background-position: -200% 0;
  }
}
</style>
