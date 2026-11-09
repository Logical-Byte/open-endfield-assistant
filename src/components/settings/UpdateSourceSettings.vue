<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { TranslationComponent } from 'i18next-vue';
import { useAppI18n } from '@/shared/i18n';
import SettingsItem from './SettingsItem.vue';
import { discardUnsavedCdk, editSettings, settingsState } from '@/features/settings/settings';
import type { UpdateSource } from '@/features/settings/types/settings';

const { t } = useAppI18n();
// 输入草稿只属于当前设置分类，失焦、切换更新源均不提交。
const cdkDraft = ref<string>('');
const clearOpen = ref<boolean>(false);
onBeforeUnmount(discardUnsavedCdk);
const ready = computed(() => (settingsState.value.status === 'ready' ? settingsState.value : null));
const savedCdk = computed<string | null>(() => ready.value?.effective.mirrorchyanCdk ?? null);
const hasSavedCdk = computed<boolean>(() => ready.value !== null && savedCdk.value !== '');
const usesOemFallback = computed<boolean>(
  () => ready.value?.effective.updateSource === 'mirrorchyan' && !hasSavedCdk.value,
);
const busy = computed<boolean>(() => !ready.value || ready.value.saving);
const items = computed(() => [
  { value: 'oem', label: 'OEM' },
  { value: 'github', label: 'GitHub' },
  { value: 'mirrorchyan', label: t('common.mirrorchyan') },
]);

watch(savedCdk, (cdk) => {
  if (cdk !== '') cdkDraft.value = '';
});

function selectSource(value: string): void {
  editSettings({ updateSource: value as UpdateSource });
}
function saveCdk(): void {
  const cdk = cdkDraft.value.trim();
  if (!busy.value && cdk) editSettings({ mirrorchyanCdk: cdk });
}
function clearCdk(): void {
  if (!ready.value || busy.value) return;
  editSettings({ mirrorchyanCdk: '' });
  clearOpen.value = false;
}
</script>

<template>
  <SettingsItem icon="i-lucide-cloud-download" :title="t('settings.update.source.title')">
    <template #description>
      <p class="text-sm text-dimmed">
        <span v-if="!ready">{{ t('settings.update.source.description') }}</span>
        <span v-else-if="usesOemFallback" class="flex items-center gap-1.5">
          <UIcon class="size-4 shrink-0 text-warning" name="i-lucide-triangle-alert" />
          <span>{{ t('settings.update.cdk.fallback') }}</span>
        </span>
        <span v-else-if="ready.effective.updateSource === 'github'">
          <TranslationComponent :translation="t('settings.update.source.githubDescription')">
            <template #releases>
              <ULink
                class="inline-flex items-center gap-1 align-baseline whitespace-nowrap"
                rel="noopener noreferrer"
                target="_blank"
                to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
              >
                GitHub Release<UIcon name="i-lucide-external-link" />
              </ULink>
            </template>
          </TranslationComponent>
        </span>
        <span v-else-if="ready?.effective.updateSource === 'mirrorchyan'">{{
          t('settings.update.source.mirrorUpdateDescription')
        }}</span>
        <span v-else>{{ t('settings.update.source.oemDescription') }}</span>
      </p>
    </template>
    <USelect
      :aria-label="t('settings.update.source.title')"
      class="w-56"
      :disabled="busy"
      :items="items"
      :model-value="ready?.effective.updateSource"
      size="sm"
      @update:model-value="selectSource"
    />
  </SettingsItem>
  <SettingsItem
    v-if="ready?.effective.updateSource === 'mirrorchyan'"
    icon="i-lucide-key-round"
    :title="t('settings.update.cdk.title')"
  >
    <template #description>
      <p class="text-sm text-dimmed">
        <TranslationComponent :translation="t('settings.update.source.mirrorDescription')">
          <template #subscribe>
            <ULink
              class="inline-flex items-center gap-1 align-baseline whitespace-nowrap"
              rel="noopener noreferrer"
              target="_blank"
              to="https://mirrorchyan.com/?source=oea"
            >
              {{ t('settings.update.cdk.subscribe') }}<UIcon name="i-lucide-external-link" />
            </ULink>
          </template>
        </TranslationComponent>
      </p>
    </template>
    <div class="flex max-w-full flex-col gap-2">
      <div class="flex max-w-full min-w-0 items-center gap-2">
        <UInput
          :aria-label="t('settings.update.cdk.title')"
          class="w-44 max-w-full min-w-0"
          :disabled="busy"
          :model-value="hasSavedCdk ? (savedCdk === null ? '••••••••' : savedCdk) : cdkDraft"
          :placeholder="t('settings.update.cdk.placeholder')"
          :readonly="hasSavedCdk"
          size="sm"
          type="password"
          @keydown.enter.prevent="saveCdk"
          @keydown.stop
          @update:model-value="cdkDraft = String($event)"
        />
        <span
          v-if="ready && (hasSavedCdk || cdkDraft)"
          class="text-xs whitespace-nowrap text-dimmed"
        >
          {{ hasSavedCdk ? t('settings.update.cdk.saved') : t('settings.update.cdk.unsaved') }}
        </span>
        <UPopover v-if="hasSavedCdk" v-model:open="clearOpen">
          <UButton
            color="error"
            :disabled="busy"
            :label="t('settings.update.cdk.clear')"
            size="sm"
            variant="outline"
          />
          <template #content>
            <div class="space-y-3 p-3">
              <p class="font-medium">{{ t('settings.update.cdk.confirmTitle') }}</p>
              <div class="flex justify-end gap-2">
                <UButton
                  color="neutral"
                  :label="t('common.cancel')"
                  size="sm"
                  variant="outline"
                  @click="clearOpen = false"
                />
                <UButton
                  color="error"
                  :disabled="busy"
                  :label="t('settings.update.cdk.confirm')"
                  size="sm"
                  @click="clearCdk"
                />
              </div>
            </div>
          </template>
        </UPopover>
        <UButton
          v-else
          :disabled="busy || !cdkDraft.trim()"
          :label="t('settings.update.cdk.save')"
          :loading="!!ready?.saving"
          size="sm"
          @click="saveCdk"
        />
      </div>
      <p v-if="ready && savedCdk === null" class="text-sm text-warning">
        {{ t('settings.update.cdk.unreadable') }}
      </p>
    </div>
  </SettingsItem>
</template>
