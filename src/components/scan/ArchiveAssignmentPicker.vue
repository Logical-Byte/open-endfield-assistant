<script setup lang="ts">
import { computed, nextTick, onMounted } from 'vue';
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
// UInputMenu 挂载后的 nextTick 会清空搜索词。保留列表外的草稿，
// 等这次内部初始化结束后，再接收正常的输入与选择事件。
let acceptingSearchUpdates = false;
onMounted((): void => {
  void nextTick((): void => {
    acceptingSearchUpdates = true;
  });
});
function updateSearch(value: string): void {
  if (acceptingSearchUpdates) search.value = value;
}
// UInputMenu 将额外属性转交给底层 ComboboxInput，显示值也从保留的草稿读取。
function displaySearch(archive: ArchiveDetails | undefined): string {
  return search.value || archive?.title || '';
}
const { t } = useAppI18n();
const selectedArchive = computed((): ArchiveDetails | undefined =>
  props.archives.find((archive): boolean => archive.id === props.selected),
);
const items = computed((): (ArchiveDetails | { type: 'label'; title: string })[] => {
  const query = search.value;
  const visible = query
    ? props.archives.filter((archive): boolean => archive.title.includes(query))
    : [...props.recommended];
  return query || !visible.length
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
      :aria-label="t('scan.chooseArchive')"
      by="id"
      class="w-full min-w-0"
      :content="{ side: 'top' }"
      :display-value="displaySearch"
      ignore-filter
      :items="items"
      label-key="title"
      :model-value="selectedArchive"
      open-on-click
      open-on-focus
      :placeholder="t('scan.chooseArchiveHint')"
      :reset-search-term-on-blur="false"
      :search-term="search"
      trailing-icon=""
      :ui="{
        base: selectedArchive && !search ? 'caret-default text-transparent!' : '',
        content: 'max-h-64 bg-elevated shadow-xl ring-accented',
      }"
      @focus="emit('focus')"
      @input="onInput"
      @update:model-value="emit('select', ($event as ArchiveDetails).id)"
      @update:search-term="updateSearch"
    >
      <template #item-label="{ item }">
        <ArchiveLabel
          :content-preview="'contentPreview' in item ? item.contentPreview : undefined"
          :title="item.title"
        />
      </template>
      <template #empty>
        <div class="space-y-2 text-left">
          <p v-if="!search" class="font-medium text-default">{{ t('scan.recommendations') }}</p>
          <p class="text-muted">
            {{ search ? t('scan.noArchives') : t('scan.noRecommendations') }}
          </p>
        </div>
      </template>
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
