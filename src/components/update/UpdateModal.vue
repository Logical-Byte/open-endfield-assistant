<script setup lang="ts">
import { ref } from 'vue';

/**
 * 更新下载弹窗（样板，纯静态）。
 *
 * 实际逻辑中，下载进度、下载源与安装状态由后端更新任务实时填充。
 */
const open = defineModel<boolean>({ required: true });

/** 下载阶段：下载中 / 安装中。 */
const phase = ref<'downloading' | 'installing'>('downloading');

/** 下载进度（0 ~ 100，样板数据）。 */
const progress = ref<number>(42);

/** 当前下载源展示名（样板数据）。 */
const downloadSource = ref('Mirror酱镜像源');
</script>

<template>
  <UModal
    v-model:open="open"
    :close="phase === 'downloading'"
    description="下载完成后将自动开始安装，请勿关闭应用"
    :dismissible="phase === 'downloading'"
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

        <template v-if="phase === 'downloading'">
          <UProgress v-model="progress" status />
          <p class="text-xs text-toned">正在下载更新包，请稍候…</p>
        </template>

        <div v-else class="flex items-center gap-3 rounded-md bg-muted p-4">
          <UIcon class="size-5 animate-spin text-primary" name="i-lucide-loader-circle" />
          <div>
            <p class="font-medium">正在安装</p>
            <p class="text-sm text-toned">更新包已下载完成，正在应用更新，请勿关闭应用。</p>
          </div>
        </div>
      </div>
    </template>

    <template #footer>
      <UButton
        v-if="phase === 'downloading'"
        color="neutral"
        label="取消下载"
        variant="outline"
        @click="open = false"
      />
      <span v-else class="text-sm text-toned">安装完成后应用将自动重启</span>
    </template>
  </UModal>
</template>
