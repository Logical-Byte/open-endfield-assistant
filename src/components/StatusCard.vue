<script setup lang="ts">
import { running } from '@/lib/appState';
</script>

<template>
  <div class="flex items-center gap-2">
    <UBadge
      :color="running ? 'success' : 'neutral'"
      :label="running ? '运行中' : '空闲'"
      variant="subtle"
    />
    <span class="text-sm text-muted">
      {{ running ? '主任务正在扫描档案库…' : '主任务待命，等待启动' }}
    </span>
  </div>
</template>
