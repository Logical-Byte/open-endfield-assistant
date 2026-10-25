<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { computed, ref, type CSSProperties } from 'vue';
import type { ScanCardState } from './scanCardState';
import type { ScannedItemView } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/shared/types/archive';
import { openImagePreview } from '@/composables/image-preview';
import ArchiveLabel from './ArchiveLabel.vue';
import ArchiveAssignmentPicker from './ArchiveAssignmentPicker.vue';
const { t } = useAppI18n();
const props = defineProps<{ item: ScannedItemView; selected?: boolean }>();
const state = defineModel<ScanCardState>('state', { required: true });
const pickerRevision = ref(0);
const emit = defineEmits<{
  choose: [id: ArchiveId | null];
  locateArchive: [id: ArchiveId];
}>();
const expanded = computed((): boolean => state.value.expanded ?? !props.item.archive);
const currentTitle = computed((): string => props.item.archive?.title ?? props.item.ocrResult);
const changed = computed(
  (): boolean =>
    state.value.editing &&
    !state.value.search &&
    (state.value.draftArchiveId !== (props.item.archive?.id ?? null) ||
      (state.value.draftTouched && props.item.manualChoice.kind === 'automatic')),
);
function beginEdit(): void {
  if (state.value.editing) return;
  state.value = {
    ...state.value,
    editing: true,
    draftArchiveId: props.item.archive?.id ?? null,
    draftTouched: false,
  };
}
function chooseArchive(id: ArchiveId | null): void {
  state.value = { ...state.value, draftArchiveId: id, draftTouched: true, search: '' };
}
function cancelEdit(): void {
  state.value = {
    ...state.value,
    editing: false,
    search: '',
    draftArchiveId: props.item.archive?.id ?? null,
    draftTouched: false,
  };
  // 放弃编辑时也重置输入控件内部的文字和弹出状态。
  pickerRevision.value++;
}
function submit(): void {
  if (!changed.value) return;
  const id = state.value.draftArchiveId;
  state.value = { ...state.value, editing: false, expanded: undefined, showOcr: false, search: '' };
  emit('choose', id);
}
// 后端截图统一为 1280×720，缩略图只裁出标题区域，完整原图仍可打开。
const cropContainerStyle: CSSProperties = { aspectRatio: '516 / 86' };
const cropImageStyle: CSSProperties = {
  width: 'calc(100% * 1280 / 516)',
  height: 'calc(100% * 720 / 86)',
  left: 'calc(100% * -360 / 516)',
  top: 'calc(100% * -48 / 86)',
};
</script>
<template>
  <UCard
    as="form"
    class="scan-card outline-2 transition-[outline-color] motion-reduce:transition-none"
    :class="
      selected && !state.editing ? 'outline-primary duration-0' : 'outline-transparent duration-400'
    "
    :ui="{
      header: 'p-0 sm:px-0',
      body: expanded ? 'p-4 sm:p-4' : 'hidden',
      footer: expanded ? 'p-2 sm:px-2' : 'hidden',
    }"
    @submit.prevent="submit"
  >
    <template #header>
      <UButton
        :aria-controls="`scan-body-${item.scannedItemId}`"
        :aria-expanded="expanded"
        class="min-h-10 w-full justify-start rounded-none px-3 py-2 text-sm ring-0"
        color="neutral"
        :icon="item.archive ? 'i-lucide-circle-check' : 'i-lucide-triangle-alert'"
        :ui="{ leadingIcon: ['size-4', item.archive ? 'text-success' : 'text-warning'] }"
        variant="ghost"
        @click="state = { ...state, expanded: !expanded }"
      >
        <span class="shrink-0 text-xs text-muted">#{{ item.scannedItemId }}</span>
        <span class="truncate">{{ item.ocrResult || t('scan.ocrEmpty') }}</span>
        <span
          v-if="state.editing"
          class="ml-auto shrink-0 text-xs font-semibold text-primary"
          role="status"
          >{{ t('scan.editing') }}</span
        >
        <UIcon
          class="ml-auto size-4 shrink-0"
          :name="expanded ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
        />
      </UButton>
    </template>
    <div :id="`scan-body-${item.scannedItemId}`" class="space-y-4">
      <div>
        <UButton
          :aria-label="t('scan.viewScreenshot')"
          class="mx-auto block w-[290px] overflow-hidden rounded p-0"
          color="neutral"
          variant="outline"
          @click="
            openImagePreview({
              url: item.image,
              name: t('scan.screenshot'),
              downloadName: `${t('scan.screenshot')} - ${currentTitle}.png`,
            })
          "
        >
          <ImagePreviewContainer class="h-[48px] w-[288px] bg-elevated" :style="cropContainerStyle">
            <img
              :alt="t('scan.screenshot')"
              class="absolute max-w-none"
              :src="item.image"
              :style="cropImageStyle"
            />
          </ImagePreviewContainer>
        </UButton>
      </div>
      <USeparator />
      <div class="space-y-4">
        <div class="space-y-1">
          <ArchiveAssignmentPicker
            v-if="!item.archive || state.editing"
            :key="pickerRevision"
            v-model:search="state.search"
            :archives="item.options"
            :recommended="item.recommendations"
            :selected="state.editing ? state.draftArchiveId : (item.archive?.id ?? null)"
            @focus="beginEdit"
            @select="chooseArchive"
          />
          <!-- 与编辑入口的 sm 按钮保持相同内边距，并预留右侧图标及间距。 -->
          <ArchiveLabel
            v-else
            :category-label="item.archive.categoryLabel"
            class="px-2.5 py-1.5 pr-8"
            :content-preview="item.archive.contentPreview"
            preview="next-line"
            :title="item.archive.title"
          />
          <div
            v-if="!item.archive || state.editing"
            class="space-y-1 rounded-md bg-elevated p-2 text-sm"
          >
            <p class="text-xs text-muted">{{ t('scan.ocr') }}</p>
            <p class="break-words">{{ item.ocrResult || t('scan.noText') }}</p>
          </div>
          <div
            v-else
            class="flex flex-wrap items-center justify-between gap-2 rounded-md bg-elevated p-2"
          >
            <p class="flex min-w-0 items-center gap-1.5 text-xs text-muted">
              <UIcon
                class="size-3.5 shrink-0"
                :name="item.manualChoice.kind === 'manual' ? 'i-lucide-hand' : 'i-lucide-scan-text'"
              />
              <span>{{
                item.manualChoice.kind === 'manual' ? t('scan.manual') : t('scan.automatic')
              }}</span>
            </p>
            <UButton
              color="neutral"
              icon="i-lucide-arrow-left"
              :label="t('scan.locateArchive')"
              size="xs"
              variant="outline"
              @click="emit('locateArchive', item.archive.id)"
            />
          </div>
        </div>

        <div
          v-if="item.archive && !state.editing && state.showOcr"
          class="rounded border border-default bg-elevated/50 p-3 text-sm break-words"
        >
          <p class="mb-1 text-xs text-muted">{{ t('scan.ocr') }}</p>
          {{ item.ocrResult || t('scan.noText') }}
        </div>
      </div>
    </div>
    <template #footer>
      <div v-if="!item.archive || state.editing" class="flex items-center justify-end gap-2">
        <UButton
          color="neutral"
          :disabled="!state.editing"
          icon="i-lucide-x"
          size="xs"
          :variant="state.editing ? 'solid' : 'outline'"
          @click="cancelEdit"
          >{{ t('scan.cancelEdit') }}</UButton
        >
        <UButton
          :color="changed ? 'primary' : 'neutral'"
          :disabled="!changed"
          icon="i-lucide-check"
          size="xs"
          type="submit"
          :variant="changed ? 'solid' : 'outline'"
          >{{ t('scan.confirmEdit') }}</UButton
        >
      </div>
      <div v-else class="flex flex-wrap items-center justify-between gap-2">
        <UButton
          :aria-expanded="state.showOcr"
          color="neutral"
          size="xs"
          :trailing-icon="state.showOcr ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
          variant="outline"
          @click="state = { ...state, showOcr: !state.showOcr }"
        >
          {{ state.showOcr ? t('scan.hideOcr') : t('scan.viewOcr') }}
        </UButton>
        <UButton
          color="neutral"
          icon="i-lucide-pencil"
          :label="t('scan.editArchive')"
          size="xs"
          variant="outline"
          @click="beginEdit"
        />
      </div>
    </template>
  </UCard>
</template>
