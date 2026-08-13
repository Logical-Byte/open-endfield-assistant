<script setup lang="ts">
import { downloadingModalOpen, installingModalOpen } from '@/utils/uiState';
import { ref } from 'vue';

const progress = ref<number>(42);
const downloadSource = ref('Mirror酱镜像源');
</script>

<template>
  <UModal
    v-model:open="downloadingModalOpen"
    description="下载完成后将自动开始安装，请勿关闭应用"
    title="下载更新"
  >
    <template #body>
      <div class="flex flex-col gap-4">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <UIcon class="text-primary" name="i-lucide-server" />
            <span class="text-sm text-toned">下载源</span>
          </div>
          <span class="text-sm font-medium">{{ downloadSource }}</span>
        </div>

        <UProgress v-model="progress" status />
        <p class="text-xs text-toned">正在下载更新包，请稍候…</p>
      </div>
    </template>

    <template #footer>
      <UButton
        color="neutral"
        label="取消下载"
        variant="outline"
        @click="downloadingModalOpen = false"
      />
      <UButton
        color="primary"
        label="测试立即安装"
        variant="solid"
        @click="
          () => {
            installingModalOpen = true;
            downloadingModalOpen = false;
          }
        "
      />
    </template>
  </UModal>
</template>
