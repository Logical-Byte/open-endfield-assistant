<script setup lang="ts">
import { computed } from 'vue';
import { useAppI18n } from '@/shared/i18n';
import { useUpdateAction } from '@/features/update/useUpdateAction';
import { showInstallModal } from '@/features/update/update';
import { automationStatus } from '@/features/automation/state';
const { t } = useAppI18n();
const { confirmation, confirm, cancel } = useUpdateAction();
const open = computed({
  get: () => !!confirmation.value || showInstallModal.value,
  set: (value: boolean) => {
    if (!value) void cancel();
  },
});
const title = computed(() => {
  if (showInstallModal.value) return t('update.install.installing');
  if (confirmation.value?.kind === 'local') return t('update.action.localTitle');
  const version = confirmation.value?.kind === 'online' ? confirmation.value.version : '';
  return t('update.action.confirmTitle', {
    version: version.startsWith('v') ? version : `v${version}`,
  });
});
</script>
<template>
  <UModal
    v-model:open="open"
    :close="false"
    :description="showInstallModal ? t('update.install.preparing') : t('update.action.warning')"
    :dismissible="!showInstallModal"
    :title="title"
    :ui="{ content: 'max-w-md', footer: 'justify-end' }"
  >
    <template v-if="showInstallModal" #body><UProgress :model-value="null" size="sm" /></template>
    <template v-if="!showInstallModal" #footer>
      <UButton color="neutral" :label="t('common.cancel')" variant="outline" @click="cancel" />
      <UTooltip :disabled="automationStatus.state === 'idle'" :text="t('update.install.afterTask')">
        <span
          ><UButton
            :disabled="automationStatus.state !== 'idle'"
            :label="t('common.confirm')"
            @click="confirm"
        /></span>
      </UTooltip>
    </template>
  </UModal>
</template>
