<script setup lang="ts">
import { ref } from 'vue';
import { exportToOem } from '@/features/archiveScan/exportOem';
defineProps<{ collected: number; total: number; unmatched: number }>();
const open = ref(false);
const exporting = ref(false);
const toast = useToast();
async function submit(): Promise<void> {
  exporting.value = true;
  const success = await exportToOem();
  exporting.value = false;
  if (success) open.value = false;
  else
    toast.add({
      title: '无法打开 OEM 导入页面',
      description: '请重试，或在日志中查看失败原因。',
      color: 'error',
    });
}
</script>
<template>
  <UTooltip text="将当前的收集状态导出到 OEM 地图集">
    <UButton
      color="neutral"
      :disabled="!total"
      icon="i-lucide-map"
      size="xs"
      variant="outline"
      @click="open = true"
      >导出收集状态</UButton
    >
  </UTooltip>
  <UModal
    v-model:open="open"
    :close="{ color: 'neutral', variant: 'outline' }"
    description="在浏览器打开 OEM 导入页面，携带当前目录的收集状态。"
    title="将当前收集状态导出到 OEM 地图集"
    :ui="{ description: 'sr-only', content: 'max-w-lg' }"
  >
    <template #body>
      <div class="space-y-5">
        <dl class="grid grid-cols-2 gap-6">
          <div>
            <dt class="mb-2 text-sm text-toned">已收集</dt>
            <dd class="text-3xl font-semibold tabular-nums">{{ collected }}</dd>
          </div>
          <div>
            <dt class="mb-2 text-sm text-toned">未收集</dt>
            <dd class="flex items-center gap-2 text-3xl font-semibold tabular-nums">
              {{ total - collected
              }}<UPopover
                v-if="unmatched > 0"
                :content="{ side: 'top', align: 'center' }"
                mode="hover"
                :ui="{ content: 'w-80 max-w-[calc(100vw-24px)] overflow-hidden' }"
              >
                <UButton
                  aria-label="查看未匹配扫描结果的导出提醒"
                  class="p-0"
                  color="warning"
                  icon="i-lucide-triangle-alert"
                  size="xs"
                  variant="link"
                />
                <template #content>
                  <UAlert
                    color="warning"
                    :description="`有 ${unmatched} 条扫描结果尚未匹配到已知档案。继续核对可能减少“未收集”的数量，让导出结果更准确。`"
                    icon="i-lucide-triangle-alert"
                    variant="soft"
                  />
                </template>
              </UPopover>
            </dd>
          </div>
        </dl>
        <UButton
          class="w-full justify-center"
          icon="i-lucide-external-link"
          :loading="exporting"
          @click="submit"
          >导出到 OEM 地图集</UButton
        >
      </div>
    </template>
  </UModal>
</template>
