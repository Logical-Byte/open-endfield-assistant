<script setup lang="ts">
import { settingsStatus } from '@/modules/settings';
import { watch } from 'vue';

const toast = useToast();

/**
 * 保存失败可能由设置页之外的编辑触发；这里常驻观察新的失败转换并只通知一次。
 * settings 模块只发布状态，不依赖任何展示层 API。
 */
watch(
  () => settingsStatus.kind,
  (kind: typeof settingsStatus.kind, previousKind: typeof settingsStatus.kind | undefined) => {
    if (kind !== 'save-error' || previousKind === 'save-error') {
      return;
    }

    toast.add({
      title: '设置未保存',
      description: '已保留当前编辑，应用仍使用最近一次成功保存的设置。',
      icon: 'i-lucide-circle-alert',
      color: 'error',
    });
  },
  { flush: 'sync' },
);
</script>

<template>
  <!-- 此组件只负责全局通知，不占用页面布局。 -->
  <span aria-hidden="true" class="hidden" />
</template>
