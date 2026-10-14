<script setup lang="ts">
import { computed } from 'vue';
import ArchiveLabel from './ArchiveLabel.vue';
import { useAppI18n } from '@/shared/i18n';
import type { ArchiveDetails } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/shared/types/archive';

const props = defineProps<{
  archives: readonly ArchiveDetails[];
  recommended: readonly ArchiveDetails[];
  selected: ArchiveId | null;
}>();
const emit = defineEmits<{ select: [id: ArchiveId | null]; focus: [] }>();
const search = defineModel<string>('search', { required: true });
const { t } = useAppI18n();
const selectedArchive = computed((): ArchiveDetails | undefined =>
  props.archives.find((archive): boolean => archive.id === props.selected),
);
const items = computed((): (ArchiveDetails | { type: 'label'; title: string })[] => {
  const prefix = search.value.toLowerCase();
  const visible = prefix
    ? props.archives.filter((archive): boolean => archive.title.toLowerCase().startsWith(prefix))
    : [...props.recommended];
  return prefix
    ? visible
    : [{ type: 'label' as const, title: t('scan.recommendations') }, ...visible];
});
// 只响应用户实际删空输入，搜索词在选择或失焦时的重置不清除档案。
function onInput(event: Event): void {
  if ((event.target as HTMLInputElement).value === '') emit('select', null);
}
</script>

<template>
  <div class="relative min-w-0">
    <UInputMenu
      v-model:search-term="search"
      :aria-label="t('scan.chooseArchive')"
      by="id"
      class="w-full min-w-0"
      :content="{ side: 'top' }"
      ignore-filter
      :items="items"
      label-key="title"
      :model-value="selectedArchive"
      open-on-click
      open-on-focus
      :placeholder="t('scan.archivePrefix')"
      trailing-icon=""
      :ui="{
        base: selectedArchive && !search ? 'caret-default text-transparent!' : '',
        content: 'max-h-64',
      }"
      @focus="emit('focus')"
      @input="onInput"
      @update:model-value="emit('select', ($event as ArchiveDetails).id)"
    >
      <template #item-label="{ item }">
        <ArchiveLabel
          :content-preview="'contentPreview' in item ? item.contentPreview : undefined"
          :title="item.title"
        />
      </template>
      <template #empty>{{ search ? t('scan.noArchives') : t('scan.noRecommendations') }}</template>
    </UInputMenu>
    <!-- 输入时显示搜索文字，其余时候在完整输入框内使用统一的档案展示。 -->
    <div
      v-if="selectedArchive && !search"
      class="pointer-events-none absolute inset-y-0 right-3 left-3 flex items-center"
    >
      <ArchiveLabel
        class="w-full"
        :content-preview="selectedArchive.contentPreview"
        :title="selectedArchive.title"
      />
    </div>
  </div>
</template>
