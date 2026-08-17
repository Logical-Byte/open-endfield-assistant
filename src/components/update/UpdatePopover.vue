<script setup lang="ts">
import {
  checkUpdate,
  downloadAndInstall,
  updatePopoverOpen,
  updateSnapshot,
} from '@/utils/app/update';
import { renderMarkdown } from '@/utils/markdown';
import { computed } from 'vue';

const visible = computed(() => !['idle', 'upToDate'].includes(updateSnapshot.value.status));
const progress = computed<number | null>(() => {
  const total = updateSnapshot.value.totalBytes;
  return total && total > 0 ? (updateSnapshot.value.downloadedBytes / total) * 100 : null;
});
const progressText = computed(() => {
  const downloaded = formatBytes(updateSnapshot.value.downloadedBytes);
  const total = updateSnapshot.value.totalBytes;
  return total === null ? downloaded : `${downloaded} / ${formatBytes(total)}`;
});
const etaText = computed<string | null>(() => {
  const { bytesPerSecond, downloadedBytes, totalBytes } = updateSnapshot.value;
  if (!totalBytes || bytesPerSecond <= 0 || downloadedBytes >= totalBytes) return null;
  const seconds = Math.ceil((totalBytes - downloadedBytes) / bytesPerSecond);
  return seconds < 60 ? `约 ${seconds} 秒` : `约 ${Math.ceil(seconds / 60)} 分钟`;
});

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  return `${value.toFixed(value >= 100 || index === 0 ? 0 : 1)} ${units[index]}`;
}
</script>

<template>
  <UPopover v-if="visible" v-model:open="updatePopoverOpen">
    <UTooltip text="应用更新">
      <div class="relative">
        <UButton
          aria-label="应用更新"
          color="neutral"
          icon="i-lucide-cloud-download"
          :loading="updateSnapshot.status === 'checking'"
          :variant="updatePopoverOpen ? 'soft' : 'ghost'"
        />
        <span
          v-if="updateSnapshot.status === 'available' || updateSnapshot.status === 'failed'"
          class="absolute top-0 right-0 size-2 rounded-full"
          :class="updateSnapshot.status === 'failed' ? 'bg-error' : 'bg-success'"
        />
      </div>
    </UTooltip>

    <template #content>
      <div class="flex max-h-[calc(100dvh-5rem)] w-md flex-col gap-3 p-4">
        <div v-if="updateSnapshot.status === 'checking'" class="flex items-center gap-2 py-2">
          <UIcon class="size-5 animate-spin text-primary" name="i-lucide-loader-circle" />
          <p class="text-sm font-medium">正在检查更新...</p>
        </div>

        <template v-else-if="updateSnapshot.status === 'failed'">
          <div class="flex items-center gap-2">
            <UIcon class="size-5 text-error" name="i-lucide-circle-alert" />
            <p class="font-semibold">更新失败</p>
          </div>
          <p class="text-sm whitespace-pre-wrap text-toned">{{ updateSnapshot.error }}</p>
          <UButton
            block
            icon="i-lucide-rotate-cw"
            :label="updateSnapshot.availableVersion ? '重试下载并安装' : '重试检查'"
            @click="updateSnapshot.availableVersion ? downloadAndInstall() : checkUpdate()"
          />
        </template>

        <template v-else>
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <UIcon class="text-xl text-primary" name="i-lucide-circle-arrow-up" />
              <p class="font-semibold">发现新版本</p>
            </div>
            <div class="flex items-center gap-1.5">
              <UBadge color="neutral" variant="subtle">v{{ updateSnapshot.currentVersion }}</UBadge>
              <UIcon class="text-toned" name="i-lucide-arrow-right" />
              <UBadge color="primary" variant="subtle"
                >v{{ updateSnapshot.availableVersion }}</UBadge
              >
            </div>
          </div>
          <div class="flex min-h-0 flex-col gap-2 rounded-md bg-muted p-3">
            <p class="text-xs font-medium text-toned">更新日志</p>
            <!-- eslint-disable vue/no-v-html -- output is sanitized by DOMPurify -->
            <div
              class="markdown-body max-h-96 min-h-0 overflow-y-auto text-sm"
              v-html="renderMarkdown(updateSnapshot.releaseNotes ?? '暂无更新日志')"
            />
            <!-- eslint-enable vue/no-v-html -->
          </div>
          <div v-if="updateSnapshot.status === 'downloading'" class="space-y-2">
            <div class="flex justify-between text-xs text-toned">
              <span>正在下载</span><span class="tabular-nums">{{ progressText }}</span>
            </div>
            <UProgress size="sm" :value="progress" />
            <div class="flex justify-between text-xs text-dimmed">
              <span>{{ formatBytes(updateSnapshot.bytesPerSecond) }}/s</span>
              <span v-if="etaText">{{ etaText }}</span>
            </div>
          </div>
          <UButton
            v-if="updateSnapshot.status === 'available'"
            block
            icon="i-lucide-download"
            label="下载并安装"
            @click="downloadAndInstall"
          />
        </template>
      </div>
    </template>
  </UPopover>
</template>
