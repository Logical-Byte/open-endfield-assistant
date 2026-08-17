<script setup lang="ts">
import {
  checkUpdate,
  downloadAndInstall,
  installUpdateModalOpen,
  updateMetadataStale,
  updateSnapshot,
} from '@/utils/app/update';
import {
  downloadEtaText,
  downloadPercentage,
  downloadProgressText,
  formatBytes,
  isUpdateInstalling,
  updateStageLabel,
} from '@/utils/update-display';
import { computed } from 'vue';

const stageLabel = computed(() => updateStageLabel(updateSnapshot.value.status));
const busy = computed(() => isUpdateInstalling(updateSnapshot.value.status));
const progress = computed(() => downloadPercentage(updateSnapshot.value));
const progressText = computed(() => downloadProgressText(updateSnapshot.value));
const etaText = computed(() => downloadEtaText(updateSnapshot.value));

function retry(): void {
  if (updateMetadataStale.value) {
    installUpdateModalOpen.value = false;
    void checkUpdate();
  } else {
    void downloadAndInstall();
  }
}
</script>

<template>
  <UModal
    v-model:open="installUpdateModalOpen"
    :close="!busy"
    :dismissible="!busy"
    title="安装更新"
  >
    <template #body>
      <div class="flex flex-col items-center gap-4 py-6 text-center">
        <UIcon
          class="size-12"
          :class="updateSnapshot.status === 'failed' ? 'text-error' : 'animate-spin text-primary'"
          :name="
            updateSnapshot.status === 'failed' ? 'i-lucide-circle-alert' : 'i-lucide-loader-circle'
          "
        />
        <div class="space-y-1">
          <p class="font-semibold">{{ stageLabel }}</p>
          <p v-if="updateSnapshot.error" class="text-sm whitespace-pre-wrap text-error">
            {{ updateSnapshot.error }}
          </p>
        </div>
        <div v-if="updateSnapshot.status === 'downloading'" class="w-full space-y-2 text-left">
          <div class="flex justify-between gap-4 text-xs text-toned">
            <span class="tabular-nums">{{ progressText }}</span>
            <span class="tabular-nums">
              {{ progress === null ? '总大小未知' : `${Math.round(progress)}%` }}
            </span>
          </div>
          <UProgress size="sm" :value="progress" />
          <div class="flex justify-between gap-4 text-xs text-dimmed">
            <span>{{ formatBytes(updateSnapshot.bytesPerSecond) }}/s</span>
            <span v-if="etaText">{{ etaText }}</span>
          </div>
        </div>
        <UProgress v-else-if="busy" class="w-full" size="sm" :value="null" />
        <div v-else class="flex gap-2">
          <UButton
            color="neutral"
            label="关闭"
            variant="soft"
            @click="installUpdateModalOpen = false"
          />
          <UButton
            v-if="updateSnapshot.availableVersion"
            :icon="updateMetadataStale ? 'i-lucide-refresh-cw' : 'i-lucide-download'"
            :label="updateMetadataStale ? '重新检查' : '重试'"
            @click="retry"
          />
        </div>
      </div>
    </template>
  </UModal>
</template>
