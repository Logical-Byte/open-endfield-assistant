<script setup lang="ts">
import { installingModalOpen } from '@/utils/uiState';
import { ref } from 'vue';

type StepStatus = 'done' | 'active' | 'pending';

interface InstallStep {
  label: string;
  status: StepStatus;
}

const steps = ref<InstallStep[]>([
  { label: '校验更新包完整性', status: 'done' },
  { label: '备份当前版本', status: 'done' },
  { label: '替换程序文件', status: 'active' },
  { label: '清理临时文件', status: 'pending' },
]);

const stepIcon: Record<StepStatus, string> = {
  done: 'i-lucide-circle-check',
  active: 'i-lucide-loader-circle',
  pending: 'i-lucide-circle',
};

const stepIconClass: Record<StepStatus, string> = {
  done: 'text-primary',
  active: 'animate-spin text-primary',
  pending: 'text-dimmed',
};
</script>

<template>
  <UModal
    v-model:open="installingModalOpen"
    :close="false"
    description="更新包已下载完成，正在应用更新"
    :dismissible="false"
    title="安装更新"
  >
    <template #body>
      <div class="flex flex-col gap-5">
        <div class="flex items-center gap-3">
          <div
            class="grid size-11 shrink-0 place-items-center rounded-full bg-accented ring ring-default"
          >
            <UIcon class="size-5 animate-spin text-primary" name="i-lucide-loader-circle" />
          </div>
          <div class="min-w-0">
            <p class="font-medium">正在安装</p>
            <p class="text-sm text-toned">请勿关闭应用或断开电源</p>
          </div>
        </div>

        <UProgress animation="swing" :model-value="null" />

        <ul class="flex flex-col gap-2.5">
          <li v-for="step in steps" :key="step.label" class="flex items-center gap-2.5 text-sm">
            <UIcon
              class="size-5 shrink-0"
              :class="stepIconClass[step.status]"
              :name="stepIcon[step.status]"
            />
            <span :class="step.status === 'pending' ? 'text-dimmed' : ''">{{ step.label }}</span>
          </li>
        </ul>
      </div>
    </template>
  </UModal>
</template>
