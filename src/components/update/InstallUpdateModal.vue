<script setup lang="ts">
import { downloadAndInstall, installUpdateModalOpen, updateSnapshot } from '@/utils/app/update';
import { computed } from 'vue';

const stageLabel = computed(() => {
  switch (updateSnapshot.value.status) {
    case 'downloading':
      return '正在下载完整更新包';
    case 'verifying':
      return '正在校验 SHA-256';
    case 'preparing':
      return '正在准备新版本资源';
    case 'bootstrapReady':
      return '准备完成，即将重启';
    case 'failed':
      return '更新失败';
    default:
      return '正在准备';
  }
});
const busy = computed(
  () => !['failed', 'available', 'idle', 'upToDate'].includes(updateSnapshot.value.status),
);
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
        <UProgress v-if="busy" class="w-full" size="sm" :value="null" />
        <div v-else class="flex gap-2">
          <UButton
            color="neutral"
            label="关闭"
            variant="soft"
            @click="installUpdateModalOpen = false"
          />
          <UButton
            v-if="updateSnapshot.availableVersion"
            icon="i-lucide-refresh-cw"
            label="重试"
            @click="downloadAndInstall"
          />
        </div>
      </div>
    </template>
  </UModal>
</template>
