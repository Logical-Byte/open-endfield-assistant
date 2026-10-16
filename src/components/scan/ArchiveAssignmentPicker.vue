<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue';
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
const inputMenu = ref<{ inputRef: HTMLInputElement | null } | null>(null);
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
async function reselectArchive(): Promise<void> {
  emit('focus');
  emit('select', null);
  await nextTick();
  inputMenu.value?.inputRef?.focus();
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
</script>

<template>
  <div class="relative min-w-0">
    <UTooltip v-if="selectedArchive && !search" :text="t('scan.reselectArchive')">
      <UButton
        :aria-label="t('scan.reselectArchive')"
        class="w-full min-w-0 justify-start"
        color="neutral"
        variant="outline"
        @click="reselectArchive"
      >
        <ArchiveLabel
          class="w-full text-left"
          :content-preview="selectedArchive.contentPreview"
          :title="selectedArchive.title"
        />
      </UButton>
    </UTooltip>
    <UInputMenu
      v-else
      ref="inputMenu"
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
      :placeholder="t('scan.chooseArchiveHint')"
      :reset-search-term-on-blur="false"
      :reset-search-term-on-select="false"
      :search-term="search"
      trailing-icon=""
      :ui="{
        content: 'max-h-64 bg-elevated shadow-xl ring-accented',
      }"
      :value="search"
      @focus="emit('focus')"
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
  </div>
</template>
