<script setup lang="ts">
import type { ScreenshotFormat } from '@/types/screenshot';
import { screenshot } from '@/utils/tauri';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';

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
const error = ref<string | null>(null);

let timer: ReturnType<typeof setTimeout> | null = null;
let capturing = false;

/** 把时间格式化为 `HH:MM:SS`。 */
function formatTime(date: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0');
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

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
    error.value = typeof err === 'string' ? err : '截图失败';
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
      <USelect v-model="fps" class="w-32" :items="fpsOptions" />
      <USelect v-model="format" class="w-28" :items="formatOptions" />

      <UButton
        v-if="!running"
        color="success"
        icon="i-lucide-play"
        label="开始监控"
        @click="startMonitor"
      />
      <UButton
        v-else
        color="error"
        icon="i-lucide-square"
        label="停止监控"
        variant="outline"
        @click="stopMonitor"
      />

      <span v-if="running" class="text-sm text-muted">
        {{ fps }} FPS<template v-if="lastCaptureAt">
          · 最近更新 {{ formatTime(lastCaptureAt) }}</template
        >
      </span>
    </div>

    <UAlert v-if="error" color="error" icon="i-lucide-circle-alert" :title="error" />

    <UCard class="min-h-0 flex-1" :ui="{ body: 'h-full p-0!' }">
      <div class="flex h-full items-center justify-center">
        <img
          v-if="imageUrl"
          alt="游戏画面监控"
          class="max-h-full max-w-full object-contain"
          :src="imageUrl"
        />
        <p v-else class="text-muted">尚未开始监控，请先启动游戏</p>
      </div>
    </UCard>
  </UContainer>
</template>
