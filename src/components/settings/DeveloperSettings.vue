<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';

import { developerSettingsEnabled as enabled } from '@/features/settings/developerSettings';

import DeveloperSettingsBody from './DeveloperSettingsBody.vue';
import SettingsCard from './SettingsCard.vue';

const { t } = useAppI18n();

const { unsupported = false } = defineProps<{ unsupported?: boolean }>();
</script>

<template>
  <SettingsCard>
    <template #header>
      <div class="flex items-center justify-between gap-4">
        <div class="flex flex-1 items-center gap-3">
          <UIcon class="text-2xl text-primary" name="i-lucide-code-2" />
          <div class="flex-1">
            <p class="font-medium">{{ t('settings.developer.section') }}</p>
            <p class="text-sm text-dimmed">{{ t('settings.developer.description') }}</p>
          </div>
        </div>
        <UButton
          color="neutral"
          :icon="enabled ? 'i-lucide-eye-off' : 'i-lucide-lock-keyhole-open'"
          :label="enabled ? t('settings.developer.close') : t('settings.developer.open')"
          size="sm"
          variant="soft"
          @click="enabled = !enabled"
        />
      </div>
    </template>
    <DeveloperSettingsBody v-if="enabled" :unsupported="unsupported" />
  </SettingsCard>
</template>
