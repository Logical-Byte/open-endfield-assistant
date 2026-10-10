<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { formatBackendError } from '@/shared/errors';
import { scanError } from '@/features/archiveScan/scannedItems';
import { computed, ref, watch, type ComputedRef, type Ref } from 'vue';

const { t } = useAppI18n();

/** 用户是否手动关闭了扫描失败提示（失败原因变化时自动恢复显示） */
const scanErrorDismissed: Ref<boolean> = ref(false);

/** 是否展示扫描失败提示（存在失败原因且未被手动关闭） */
const showScanError: ComputedRef<boolean> = computed(
  (): boolean => scanError.value !== null && !scanErrorDismissed.value,
);

/** 当前扫描失败原因（无失败时为 undefined，用于提示文案） */
const scanErrorMessage: ComputedRef<string | undefined> = computed((): string | undefined =>
  scanError.value ? formatBackendError(scanError.value) : undefined,
);

// 失败原因变化（含重新失败）时恢复显示提示
watch(scanError, () => {
  scanErrorDismissed.value = false;
});
</script>

<template>
  <UAlert
    v-if="showScanError"
    close
    color="error"
    :description="scanErrorMessage"
    icon="i-lucide-circle-alert"
    orientation="horizontal"
    :title="t('scan.failed')"
    variant="outline"
    @update:open="scanErrorDismissed = true"
  >
    <template #actions>
      <UButton
        color="info"
        icon="i-lucide-scroll-text"
        :label="t('scan.logs')"
        size="sm"
        to="/log"
        variant="outline"
      />
    </template>
  </UAlert>
</template>
