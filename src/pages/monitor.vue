<script setup lang="ts">
import type { ScreenshotFormat } from '@/features/monitor/types/screenshot';
import { screenshot } from '@/features/monitor/ipc';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useAppI18n } from '@/shared/i18n';
import { formatBackendError, normalizeBackendError } from '@/shared/errors';
import type { ErrorFacts } from '@/shared/errors';

const { t, d, n } = useAppI18n();

/** 监控截图分辨率（720p）。 */
const SCREENSHOT_WIDTH = 1280;
const SCREENSHOT_HEIGHT = 720;

/** 截图编码格式选项（jpeg 体积小，适合实时预览；png 无损但体积大；webp 折中）。 */
const formatOptions: { label: string; value: ScreenshotFormat }[] = [
  { label: 'JPEG', value: 'jpeg' },
  { label: 'PNG', value: 'png' },
  { label: 'WebP', value: 'webp' },
];

/** 帧率选项（fps），默认 1 帧/秒。 */
const fpsOptions = [
  { label: '0.5 FPS', value: 0.5 },
  { label: '1 FPS', value: 1 },
  { label: '2 FPS', value: 2 },
  { label: '5 FPS', value: 5 },
  { label: '10 FPS', value: 10 },
  { label: '30 FPS', value: 30 },
];

const fps = ref(1);
const format = ref<ScreenshotFormat>('jpeg');
const running = ref(false);
const imageUrl = ref<string | null>(null);
const lastCaptureAt = ref<Date | null>(null);
const error = ref<ErrorFacts | null>(null);
const errorSummary = computed(() => (error.value ? formatBackendError(error.value) : null));
const monitoringSummary = computed(() =>
  lastCaptureAt.value
    ? t('monitor.updated', {
        fps: n(fps.value, 'quantity'),
        time: d(lastCaptureAt.value, 'dateTime'),
      })
    : t('monitor.running', { fps: n(fps.value, 'quantity') }),
);

let timer: ReturnType<typeof setTimeout> | null = null;
let capturing = false;

/** 截图一次并更新画面（重入保护：上一帧未完成时跳过本轮，避免积压）。 */
async function captureOnce(): Promise<void> {
  if (capturing) {
    return;
  }
  capturing = true;
  try {
    const data = await screenshot(SCREENSHOT_WIDTH, SCREENSHOT_HEIGHT, format.value);
    imageUrl.value = `data:image/${format.value};base64,${data}`;
    lastCaptureAt.value = new Date();
    error.value = null;
  } catch (err) {
    error.value = normalizeBackendError(err, 'screenshot');
  } finally {
    capturing = false;
  }
}

/** 按当前帧率调度下一帧。 */
function scheduleNext(): void {
  timer = setTimeout(tick, Math.round(1000 / fps.value));
}

/** 执行一帧并调度下一帧。 */
function tick(): void {
  if (!running.value) {
    return;
  }
  void captureOnce();
  scheduleNext();
}

/** 开始监控（按当前帧率循环截图）。 */
function startMonitor(): void {
  if (running.value) {
    return;
  }
  running.value = true;
  error.value = null;
  tick();
}

/** 停止监控。 */
function stopMonitor(): void {
  running.value = false;
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
}

// 帧率变化时，按新帧率重新调度下一帧
watch(fps, () => {
  if (!running.value) {
    return;
  }
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
  scheduleNext();
});

// 打开监控页面即自动开始监控
onMounted(startMonitor);

// 离开页面时停止监控
onBeforeUnmount(stopMonitor);
</script>

<template>
  <UContainer class="flex h-full flex-col gap-4 py-4">
    <div class="flex flex-wrap items-center gap-2">
      <USelect
        v-model="fps"
        :aria-label="t('monitor.frameRate')"
        class="w-32"
        :items="fpsOptions"
      />
      <USelect
        v-model="format"
        :aria-label="t('monitor.format')"
        class="w-28"
        :items="formatOptions"
      />

      <UButton
        v-if="!running"
        color="success"
        icon="i-lucide-play"
        :label="t('monitor.start')"
        @click="startMonitor"
      />
      <UButton
        v-else
        color="error"
        icon="i-lucide-square"
        :label="t('monitor.stop')"
        @click="stopMonitor"
      />

      <span v-if="running" class="text-sm text-muted">
        {{ monitoringSummary }}
      </span>
    </div>

    <UAlert
      v-if="errorSummary"
      :actions="[
        { label: t('monitor.openLogs'), to: '/logs', icon: 'i-lucide-file-text', variant: 'link' },
      ]"
      color="error"
      icon="i-lucide-circle-alert"
      :title="errorSummary"
      variant="subtle"
    />

    <UCard class="min-h-0 flex-1" :ui="{ body: 'h-full p-0!' }">
      <div class="flex h-full items-center justify-center">
        <img
          v-if="imageUrl"
          :alt="t('monitor.imageAlt')"
          class="max-h-full max-w-full object-contain"
          :src="imageUrl"
        />
        <p v-else class="text-muted">{{ t('monitor.empty') }}</p>
      </div>
    </UCard>
  </UContainer>
</template>
