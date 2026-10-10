<script setup lang="ts">
import { useTranslatedToast } from '@/shared/i18n/toast';
import { useAppI18n } from '@/shared/i18n';
import { ref } from 'vue';
import { exportToOem } from '@/features/archiveScan/exportOem';
defineProps<{ collected: number; total: number; unmatched: number }>();
const { t, n } = useAppI18n();

const open = ref(false);
const exporting = ref(false);
const toast = useTranslatedToast();
async function submit(): Promise<void> {
  exporting.value = true;
  const success = await exportToOem();
  exporting.value = false;
  if (success) open.value = false;
  else
    toast.add({ color: 'error' }, () => ({
      title: t('scan.exportFailedTitle'),
      description: t('scan.exportFailedDescription'),
    }));
}
</script>
<template>
  <UTooltip :text="t('scan.exportTooltip')">
    <UButton
      color="neutral"
      :disabled="!total"
      icon="i-lucide-map"
      size="xs"
      variant="outline"
      @click="open = true"
      >{{ t('scan.export') }}</UButton
    >
  </UTooltip>
  <UModal
    v-model:open="open"
    :close="{ color: 'neutral', variant: 'outline' }"
    :description="t('scan.exportDescription')"
    :title="t('scan.exportTitle')"
    :ui="{ description: 'sr-only', content: 'max-w-lg' }"
  >
    <template #body>
      <div class="space-y-5">
        <dl class="grid grid-cols-2 gap-6">
          <div>
            <dt class="mb-2 text-sm text-toned">{{ t('scan.collected') }}</dt>
            <dd class="text-3xl font-semibold tabular-nums">{{ n(collected) }}</dd>
          </div>
          <div>
            <dt class="mb-2 text-sm text-toned">{{ t('scan.notCollected') }}</dt>
            <dd class="flex items-center gap-2 text-3xl font-semibold tabular-nums">
              {{ n(total - collected)
              }}<UPopover
                v-if="unmatched > 0"
                :content="{ side: 'top', align: 'center' }"
                mode="hover"
                :ui="{ content: 'w-80 max-w-[calc(100vw-24px)] overflow-hidden' }"
              >
                <UButton
                  :aria-label="t('scan.exportWarningLabel')"
                  class="p-0"
                  color="warning"
                  icon="i-lucide-triangle-alert"
                  size="xs"
                  variant="link"
                />
                <template #content>
                  <UAlert
                    color="warning"
                    :description="t('scan.exportWarning', { count: unmatched })"
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
          >{{ t('scan.exportSubmit') }}</UButton
        >
      </div>
    </template>
  </UModal>
</template>
