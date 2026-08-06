<script setup lang="ts">
import { useAppState } from '@/composables/app/useAppState';

const { appStatus } = useAppState();
</script>

<template>
  <div class="flex items-center gap-2">
    <UBadge
      :color="appStatus.running ? 'success' : 'neutral'"
      :label="appStatus.running ? '运行中' : '空闲'"
      variant="subtle"
    />
    <span class="text-sm text-muted">
      {{ appStatus.running ? '主任务正在扫描档案库…' : '主任务待命，等待启动' }}
    </span>
  </div>
</template>
