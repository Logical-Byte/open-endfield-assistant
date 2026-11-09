<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useAppI18n } from '@/shared/i18n';
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
const busy = computed<boolean>(() => !ready.value || ready.value.saving);
const canSelectMirror = computed<boolean>(() => !!savedCdk.value);
const items = computed(() => [
  { value: 'mirrorchyan', label: t('common.mirrorchyan'), disabled: !canSelectMirror.value },
  { value: 'oem', label: 'OEM' },
  { value: 'github', label: 'GitHub' },
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
  editSettings({
    mirrorchyanCdk: '',
    updateSource:
      ready.value.effective.updateSource === 'mirrorchyan'
        ? 'oem'
        : ready.value.effective.updateSource,
  });
  clearOpen.value = false;
}
</script>

<template>
  <div class="space-y-3">
    <p class="font-medium">{{ t('settings.update.source.title') }}</p>
    <URadioGroup
      :aria-label="t('settings.update.source.title')"
      :disabled="busy"
      :items="items"
      :model-value="ready?.effective.updateSource"
      size="sm"
      :ui="{
        item: 'rounded-lg border border-default p-3 has-[[data-state=checked]]:border-primary',
        wrapper: 'w-full',
        description: 'mt-2',
      }"
      variant="list"
      @update:model-value="selectSource"
    >
      <template #label="{ item }">
        <UTooltip
          v-if="item.value === 'mirrorchyan'"
          :disabled="canSelectMirror || !ready"
          :text="
            savedCdk === null
              ? t('settings.update.cdk.unreadable')
              : t('settings.update.cdk.required')
          "
        >
          <span :tabindex="!canSelectMirror && ready ? 0 : -1">{{ item.label }}</span>
        </UTooltip>
        <span v-else>{{ item.label }}</span>
      </template>
      <template #description="{ item }">
        <span v-if="item.value === 'mirrorchyan'" class="flex flex-col gap-2">
          <span class="flex flex-wrap items-center gap-2">
            <UInput
              :aria-label="t('settings.update.cdk.title')"
              as="span"
              class="min-w-32 flex-1"
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
            <span v-if="ready && (hasSavedCdk || cdkDraft)" class="text-xs text-dimmed">
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
          </span>
          <span class="flex justify-end">
            <UTooltip :text="t('settings.update.cdk.service')">
              <ULink
                class="inline-flex items-center gap-1 whitespace-nowrap"
                rel="noopener noreferrer"
                target="_blank"
                to="https://mirrorchyan.com/?source=oea"
              >
                {{ t('settings.update.cdk.subscribe') }}<UIcon name="i-lucide-external-link" />
              </ULink>
            </UTooltip>
          </span>
          <span v-if="ready && savedCdk === null" class="text-warning">{{
            t('settings.update.cdk.unreadable')
          }}</span>
        </span>
      </template>
    </URadioGroup>
  </div>
</template>
