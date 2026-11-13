<script setup lang="ts">
import SidebarItem from '@/components/layout/SidebarItem.vue';
import { useUpdateAction } from '@/features/update/useUpdateAction';
import UpdateActionIcon from './UpdateActionIcon.vue';
import { computed, ref, watch } from 'vue';
import { renderMarkdown } from '@/shared/markdown';
import { useAppI18n } from '@/shared/i18n';
defineProps<{ collapsed: boolean }>();
const {
  phase,
  release,
  version,
  icon,
  spinning,
  progress,
  label,
  tooltip,
  disabledReason,
  activate,
} = useUpdateAction();
const { t } = useAppI18n();
const releaseNotesOpen = ref(false);
const showReleaseNotes = computed(
  () => release.value !== null && (phase.value === 'available' || phase.value === 'ready'),
);
watch(phase, () => {
  releaseNotesOpen.value = false;
});
</script>
<template>
  <UPopover
    :close-delay="150"
    :content="{ side: 'right', align: 'end', sideOffset: 12, collisionPadding: 12 }"
    mode="hover"
    :open="showReleaseNotes && releaseNotesOpen"
    :open-delay="150"
    :ui="{
      content: 'flex max-h-[calc(100dvh-2rem)] w-80 max-w-[calc(100vw-2rem)] flex-col gap-3 p-4',
    }"
    @update:open="releaseNotesOpen = showReleaseNotes && $event"
  >
    <UTooltip
      :content="{ side: 'right' }"
      :disabled="collapsed || showReleaseNotes"
      :text="tooltip"
    >
      <SidebarItem
        :aria-disabled="!!disabledReason"
        :collapsed="collapsed"
        :icon="icon"
        :label="label"
        :tooltip="tooltip"
        :tooltip-disabled="showReleaseNotes"
        @click="
          releaseNotesOpen = false;
          activate();
        "
      >
        <template #icon
          ><UpdateActionIcon
            :icon="icon"
            :progress="progress?.progress ?? null"
            :ready="phase === 'ready'"
            :spinning="spinning"
        /></template>
      </SidebarItem>
    </UTooltip>
    <template #content>
      <div class="flex items-baseline justify-between gap-3">
        <p class="text-sm font-medium">{{ version }}</p>
        <p class="text-xs text-muted">{{ t('update.releaseNotes.title') }}</p>
      </div>
      <p v-if="disabledReason" class="text-xs text-muted">{{ disabledReason }}</p>
      <!-- eslint-disable vue/no-v-html 渲染结果经 DOMPurify 消毒 -->
      <div
        class="markdown-body max-h-[min(20rem,calc(100dvh-8rem))] min-h-0 scrollbar-gutter-stable overflow-y-auto break-words"
        v-html="renderMarkdown(release?.releaseNote || t('update.releaseNotes.empty'))"
      />
      <!-- eslint-enable vue/no-v-html -->
    </template>
  </UPopover>
</template>
