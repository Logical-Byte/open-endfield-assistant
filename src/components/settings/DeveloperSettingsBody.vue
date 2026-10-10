<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { simulateArchiveScan } from '@/features/archiveScan/workerType';
import {
  developerInstallBusy,
  developerInstallTrace,
  developerInstallUnavailable,
  developerInstallUpdatePackage,
} from '@/features/update/developerUpdate';

import SettingsItem from './SettingsItem.vue';

const { t } = useAppI18n();

const { unsupported = false } = defineProps<{ unsupported?: boolean }>();
</script>

<template>
  <div>
    <UAlert
      color="warning"
      icon="i-lucide-triangle-alert"
      :title="t('settings.developer.warning')"
      variant="subtle"
    />
  </div>
  <SettingsItem
    :description="t('settings.developer.simulate.description')"
    icon="i-lucide-scan-text"
    :title="t('settings.developer.simulate.title')"
  >
    <UCheckbox
      v-model="simulateArchiveScan"
      color="warning"
      :label="t('settings.developer.simulate.enable')"
    />
  </SettingsItem>
  <SettingsItem
    :description="t('settings.developer.install.description')"
    icon="i-lucide-flask-conical"
    :title="t('settings.developer.install.title')"
  >
    <UBadge
      v-if="unsupported"
      color="neutral"
      :label="t('settings.unsupported.label')"
      variant="soft"
    />
    <div v-else class="flex w-96 flex-col items-end gap-2">
      <UButton
        color="warning"
        :disabled="developerInstallUnavailable"
        icon="i-lucide-package-open"
        :label="t('settings.developer.install.button')"
        :loading="developerInstallBusy"
        @click="developerInstallUpdatePackage"
      />
      <pre
        class="max-h-52 w-full overflow-auto rounded-md bg-muted p-2 text-xs whitespace-pre-wrap text-toned"
        >{{
          developerInstallTrace.length > 0
            ? developerInstallTrace.join('\n')
            : t('settings.developer.trace.empty')
        }}</pre>
    </div>
  </SettingsItem>
</template>
