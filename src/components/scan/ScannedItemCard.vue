<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { computed } from 'vue';
import { useEvidencePopover } from './useEvidencePopover';
import type { ScanCardState } from './scanCardState';
import type { ScannedItemView } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/shared/types/archive';
import { openImagePreview } from '@/composables/image-preview';
import type { CSSProperties } from 'vue';
import ArchiveLabel from './ArchiveLabel.vue';
import ArchiveAssignmentPicker from './ArchiveAssignmentPicker.vue';
const { t } = useAppI18n();
const props = defineProps<{ item: ScannedItemView; selected?: boolean }>();
const state = defineModel<ScanCardState>('state', { required: true });
const emit = defineEmits<{
  choose: [id: ArchiveId | null];
  locateArchive: [id: ArchiveId];
  clearHighlight: [];
}>();
const {
  open: floatingMatchesOpen,
  cancelClose,
  scheduleClose,
} = useEvidencePopover((): void => emit('clearHighlight'));
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
  <article
    class="scan-card overflow-hidden rounded-lg border border-default bg-default"
    :class="selected && !state.editing && 'ring-2 ring-primary'"
  >
    <UButton
      :aria-controls="`scan-body-${item.scannedItemId}`"
      :aria-expanded="expanded"
      class="w-full justify-start rounded-none py-2 ring-0"
      :class="[
        state.editing && 'bg-primary/25 hover:bg-primary/30',
        expanded && 'border-b border-default',
      ]"
      :color="state.editing ? 'primary' : 'neutral'"
      :icon="item.archive ? 'i-lucide-circle-check' : 'i-lucide-triangle-alert'"
      :ui="{ leadingIcon: item.archive ? 'text-success' : 'text-warning' }"
      :variant="state.editing ? 'soft' : 'subtle'"
      @click="state = { ...state, expanded: !expanded }"
    >
      <span class="shrink-0 text-xs text-muted">#{{ item.scannedItemId }}</span>
      <span class="truncate">{{ currentTitle || t('scan.ocrEmpty') }}</span>
      <span
        v-if="state.editing"
        class="ml-auto shrink-0 text-xs font-semibold text-primary"
        role="status"
        >{{ t('scan.editing') }}</span
      >
      <UIcon
        class="ml-auto shrink-0"
        :name="expanded ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
      />
    </UButton>
    <div v-show="expanded" :id="`scan-body-${item.scannedItemId}`" class="space-y-2 p-2">
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
      <div
        v-if="!item.archive || state.editing"
        class="flex items-start gap-3 rounded border border-default bg-elevated/50 px-3 py-2 text-sm"
      >
        <span class="shrink-0 text-xs leading-5 text-muted">{{ t('scan.ocr') }}</span>
        <p class="min-w-0 break-words">{{ item.ocrResult || t('scan.noText') }}</p>
      </div>
      <form
        v-if="!item.archive || state.editing"
        class="space-y-2 p-2 transition-colors"
        :class="state.editing ? 'rounded-md bg-primary/20' : 'border-t border-default'"
        @submit.prevent="submit"
      >
        <ArchiveAssignmentPicker
          v-model:search="state.search"
          :archives="item.options"
          :recommended="item.recommendations"
          :selected="state.editing ? state.draftArchiveId : (item.archive?.id ?? null)"
          @focus="beginEdit"
          @select="chooseArchive"
        />
        <div class="flex items-center justify-end gap-2">
          <UButton
            color="neutral"
            :disabled="!state.editing"
            variant="outline"
            @click="cancelEdit"
            >{{ t('scan.cancelEdit') }}</UButton
          >
          <UButton :disabled="!changed" icon="i-lucide-check" type="submit">{{
            t('scan.confirmEdit')
          }}</UButton>
        </div>
      </form>
      <div v-else class="space-y-3 border-t border-default pt-3">
        <div class="rounded border border-success/30 bg-success/5 p-3">
          <UButton
            :aria-label="t('scan.editArchive')"
            class="float-right"
            color="neutral"
            icon="i-lucide-pencil"
            size="xs"
            variant="outline"
            @click="beginEdit"
          />
          <p class="mb-1 text-xs text-muted">
            {{ item.manualChoice.kind === 'manual' ? t('scan.manual') : t('scan.automatic') }}
          </p>
          <ArchiveLabel
            class="text-base"
            :content-preview="item.archive.contentPreview"
            :title="item.archive.title"
          />
          <div class="mt-2 flex items-center justify-between gap-1">
            <span class="text-xs text-muted">{{ item.categoryLabel }}</span>
            <UPopover
              v-model:open="floatingMatchesOpen"
              :content="{ side: 'bottom', align: 'end', sideOffset: 5 }"
              mode="click"
              :ui="{
                content: 'w-80 max-w-[calc(100vw-24px)] bg-elevated p-3 shadow-xl ring-accented',
              }"
            >
              <UButton
                :aria-expanded="floatingMatchesOpen"
                class="h-7 w-32 justify-center"
                color="neutral"
                icon="i-lucide-link"
                :label="t('scan.matches', { count: 1 })"
                size="xs"
                trailing-icon="i-lucide-chevron-down"
                variant="outline"
                @mouseenter="cancelClose"
                @mouseleave="scheduleClose"
              />
              <template #content>
                <div @mouseenter="cancelClose" @mouseleave="scheduleClose">
                  <p class="mb-2 text-xs text-muted">
                    {{ t('scan.linkedSummary', { id: item.scannedItemId, count: 1 }) }}
                  </p>
                  <div
                    class="flex items-center justify-between gap-2 rounded border border-default px-2 py-1.5"
                  >
                    <div class="min-w-0">
                      <ArchiveLabel
                        :content-preview="item.archive.contentPreview"
                        :title="item.archive.title"
                      />
                      <p class="truncate font-mono text-[10px] text-dimmed">
                        {{ item.archive.id }}
                      </p>
                    </div>
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
              </template>
            </UPopover>
          </div>
        </div>
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
        <div
          v-if="state.showOcr"
          class="rounded border border-default bg-elevated/50 p-3 text-sm break-words"
        >
          <p class="mb-1 text-xs text-muted">{{ t('scan.ocr') }}</p>
          {{ item.ocrResult || t('scan.noText') }}
        </div>
      </div>
    </div>
  </article>
</template>
