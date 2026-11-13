<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { useUpdateAction } from '@/features/update/useUpdateAction';
import { appPresentation } from '@/app/presentation';
import { AppVariant } from '@/app/build';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onMounted, onUnmounted, ref } from 'vue';

const { t } = useAppI18n();

// 应用图标：使用 /favicon.ico（dev 由 vite 中间件提供，构建后位于 dist 根目录）。
// 用动态绑定避免 Vite 把它当作模块导入解析。
const faviconUrl = '/favicon.ico';

const { release, label, tooltip, disabledReason, activate } = useUpdateAction();

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
    class="flex h-(--ui-title-height) shrink-0 items-center justify-between border-b border-default bg-default select-none"
    data-tauri-drag-region
  >
    <div class="flex h-full items-center gap-1.5 px-3" data-tauri-drag-region>
      <img
        alt="OEA"
        class="pointer-events-none size-4 object-contain"
        draggable="false"
        :src="faviconUrl"
      />
      <button
        v-if="release"
        :aria-label="tooltip"
        class="text-xs font-medium text-toned hover:text-primary"
        data-tauri-drag-region="false"
        :disabled="!!disabledReason"
        @click="activate"
      >
        {{ label }}
      </button>
      <span class="pointer-events-none font-ui text-xs text-toned">
        {{ appPresentation.name }}
        <span v-if="appPresentation.variant === AppVariant.Release">
          v{{ appPresentation.version }}
        </span>
      </span>
    </div>

    <div class="flex h-full">
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-accented hover:text-toned"
        :title="t('window.minimize')"
        type="button"
        @click="appWindow?.minimize"
      >
        <UIcon class="size-4" name="i-lucide-minus" />
      </button>
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-accented hover:text-toned"
        :title="isMaximized ? t('window.restore') : t('window.maximize')"
        type="button"
        @click="appWindow?.toggleMaximize"
      >
        <UIcon v-if="isMaximized" class="size-3 rotate-180" name="i-lucide-copy" />
        <UIcon v-else class="size-3" name="i-lucide-square" />
      </button>
      <button
        class="flex h-full w-12 items-center justify-center text-muted transition-colors hover:bg-red-500 hover:text-white"
        :title="t('common.close')"
        type="button"
        @click="appWindow?.close"
      >
        <UIcon class="size-4" name="i-lucide-x" />
      </button>
    </div>
  </div>
</template>
