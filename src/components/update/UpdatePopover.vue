<script setup lang="ts">
import UpdateDownloadSettings from '@/components/update/UpdateDownloadSettings.vue';
import {
  checkUpdate,
  downloadAndInstall,
  updateMetadataStale,
  updatePopoverOpen,
  updateSnapshot,
} from '@/utils/app/update';
import { saving } from '@/utils/app/config';
import { renderMarkdown } from '@/utils/markdown';
import {
  downloadEtaText,
  downloadPercentage,
  downloadProgressText,
  formatBytes,
  isUpdatePopoverVisible,
  needsUpdateAttention,
} from '@/utils/update-display';
import { computed } from 'vue';

const visible = computed(() => isUpdatePopoverVisible(updateSnapshot.value.status));
const attention = computed(() => needsUpdateAttention(updateSnapshot.value.status));
const progress = computed(() => downloadPercentage(updateSnapshot.value));
const progressText = computed(() => downloadProgressText(updateSnapshot.value));
const etaText = computed(() => downloadEtaText(updateSnapshot.value));
const canInstall = computed(
  () => updateSnapshot.value.availableVersion !== null && !updateMetadataStale.value,
);
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
          v-if="attention"
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
          <UpdateDownloadSettings />
          <UButton
            block
            :disabled="saving"
            icon="i-lucide-rotate-cw"
            :label="canInstall ? '重试下载并安装' : '重新检查'"
            @click="canInstall ? downloadAndInstall() : checkUpdate()"
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
          <UpdateDownloadSettings />
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
            v-if="updateSnapshot.status === 'available' && updateMetadataStale"
            block
            :disabled="saving"
            icon="i-lucide-refresh-cw"
            label="重新检查以应用下载源"
            @click="checkUpdate"
          />
          <UButton
            v-else-if="updateSnapshot.status === 'available'"
            block
            :disabled="saving"
            icon="i-lucide-download"
            label="下载并安装"
            @click="downloadAndInstall"
          />
        </template>
      </div>
    </template>
  </UPopover>
</template>
