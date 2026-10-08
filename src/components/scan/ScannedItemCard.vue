<script setup lang="ts">
import { computed } from 'vue';
import { useEvidencePopover } from './useEvidencePopover';
import type { ScanCardState } from './scanCardState';
import type { ScannedItemView } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/features/archiveScan/types/scannedItem';
import { openImagePreview } from '@/composables/image-preview';
import type { CSSProperties } from 'vue';
const props = defineProps<{ item: ScannedItemView; selected?: boolean }>();
const state = defineModel<ScanCardState>('state', { required: true });
const emit = defineEmits<{
  correct: [title: string];
  locateArchive: [id: ArchiveId];
  clearHighlight: [];
}>();
const {
  open: floatingMatchesOpen,
  cancelClose,
  scheduleClose,
} = useEvidencePopover((): void => emit('clearHighlight'));
const expanded = computed((): boolean => state.value.expanded ?? !props.item.archives.length);
const currentTitle = computed((): string => props.item.correctedTitle ?? props.item.ocrResult);
const changed = computed(
  (): boolean =>
    state.value.editing &&
    !!state.value.draft.trim() &&
    state.value.draft.trim() !== currentTitle.value,
);
function beginEdit(): void {
  state.value = { ...state.value, editing: true };
}
function cancelEdit(): void {
  state.value = { ...state.value, editing: false, draft: currentTitle.value };
}
function submit(): void {
  if (!changed.value) return;
  const title = state.value.draft.trim();
  state.value = {
    ...state.value,
    editing: false,
    expanded: undefined,
    showOcr: false,
    draft: title,
  };
  emit('correct', title);
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
      class="w-full justify-start rounded-none py-2"
      :class="state.editing && 'bg-primary/25 hover:bg-primary/30'"
      :color="state.editing ? 'primary' : 'neutral'"
      :icon="item.archives.length ? 'i-lucide-circle-check' : 'i-lucide-triangle-alert'"
      :ui="{ leadingIcon: item.archives.length ? 'text-success' : 'text-warning' }"
      :variant="state.editing ? 'soft' : 'subtle'"
      @click="state = { ...state, expanded: !expanded }"
    >
      <span class="shrink-0 text-xs text-muted">#{{ item.scannedItemId }}</span>
      <span class="truncate">{{ currentTitle || 'OCR 未识别到文字' }}</span>
      <span
        v-if="state.editing"
        class="ml-auto shrink-0 text-xs font-semibold text-primary"
        role="status"
        >编辑中</span
      >
      <UIcon
        class="ml-auto shrink-0"
        :name="expanded ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
      />
    </UButton>
    <div v-show="expanded" :id="`scan-body-${item.scannedItemId}`" class="space-y-2 p-2">
      <div>
        <UButton
          aria-label="查看完整档案截图"
          class="mx-auto block w-[290px] overflow-hidden rounded p-0"
          color="neutral"
          variant="outline"
          @click="
            openImagePreview({
              url: item.image,
              name: '档案详情截图',
              downloadName: `档案详情截图 - ${currentTitle}.png`,
            })
          "
        >
          <ImagePreviewContainer class="h-[48px] w-[288px] bg-elevated" :style="cropContainerStyle">
            <img
              alt="档案详情截图"
              class="absolute max-w-none"
              :src="item.image"
              :style="cropImageStyle"
            />
          </ImagePreviewContainer>
        </UButton>
      </div>
      <div
        v-if="!item.archives.length || state.editing"
        class="flex items-start gap-3 rounded border border-default bg-elevated/50 px-3 py-2 text-sm"
      >
        <span class="shrink-0 text-xs leading-5 text-muted">OCR 结果</span>
        <p class="min-w-0 break-words">{{ item.ocrResult || '未识别到文字' }}</p>
      </div>
      <form
        v-if="!item.archives.length || state.editing"
        class="space-y-2 p-2 transition-colors"
        :class="state.editing ? 'rounded-md bg-primary/20' : 'border-t border-default'"
        @submit.prevent="submit"
      >
        <div class="flex justify-end">
          <UTooltip :content="{ side: 'top' }" text="输入或选择与这个截图匹配的档案标题"
            ><UIcon class="size-4 shrink-0" name="i-lucide-circle-help" tabindex="0"
          /></UTooltip>
        </div>
        <UInputMenu
          :id="`title-${item.scannedItemId}`"
          aria-label="匹配到目录中的标题"
          class="w-full"
          :content="{ side: 'top' }"
          :items="[...item.candidates]"
          mode="autocomplete"
          :model-value="state.draft"
          placeholder="输入或选择正确标题"
          :ui="{ content: 'max-h-48' }"
          @focus="beginEdit"
          @update:model-value="state = { ...state, draft: $event, editing: true }"
        >
          <template #empty>没有找到对应标题，可继续输入</template>
        </UInputMenu>
        <div class="flex items-center justify-end gap-2">
          <UButton color="neutral" :disabled="!state.editing" variant="outline" @click="cancelEdit"
            >放弃编辑</UButton
          >
          <UButton
            :color="changed ? 'primary' : 'neutral'"
            :disabled="!changed"
            icon="i-lucide-check"
            type="submit"
            :variant="changed ? 'solid' : 'outline'"
            >确认编辑</UButton
          >
        </div>
      </form>
      <div v-else class="space-y-3 border-t border-default pt-3">
        <div class="rounded border border-success/30 bg-success/5 p-3">
          <UButton
            aria-label="编辑匹配标题"
            class="float-right"
            color="neutral"
            icon="i-lucide-pencil"
            size="xs"
            variant="outline"
            @click="beginEdit"
          />
          <p class="mb-1 text-xs text-muted">
            {{ item.manuallyCorrected ? '被人工纠正的结果' : '由 OCR 自动匹配的结果' }}
          </p>
          <p class="text-base font-medium break-words">{{ currentTitle }}</p>
          <div class="mt-2 flex items-center justify-between gap-1">
            <span class="text-xs text-muted">{{ item.categoryLabel }}</span>
            <UPopover
              v-model:open="floatingMatchesOpen"
              :content="{ side: 'bottom', align: 'end', sideOffset: 5 }"
              mode="click"
              :ui="{ content: 'w-80 max-w-[calc(100vw-24px)] p-3' }"
              ><UButton
                :aria-expanded="floatingMatchesOpen"
                class="h-7 w-32 justify-center"
                color="neutral"
                icon="i-lucide-link"
                :label="`${item.archives.length} 份关联档案`"
                size="xs"
                trailing-icon="i-lucide-chevron-down"
                variant="outline"
                @mouseenter="cancelClose"
                @mouseleave="scheduleClose" /><template #content
                ><div @mouseenter="cancelClose" @mouseleave="scheduleClose">
                  <p class="mb-2 text-xs text-muted">
                    扫描 #{{ item.scannedItemId }} 关联了 {{ item.archives.length }} 份档案
                  </p>
                  <div class="max-h-64 space-y-1.5 overflow-y-auto">
                    <div
                      v-for="archive in item.archives"
                      :key="archive.id"
                      class="flex items-center justify-between gap-2 rounded border border-default px-2 py-1.5"
                    >
                      <div class="min-w-0">
                        <p class="text-xs">{{ archive.title }}</p>
                        <p class="truncate font-mono text-[10px] text-dimmed">{{ archive.id }}</p>
                      </div>
                      <UButton
                        color="neutral"
                        icon="i-lucide-arrow-left"
                        label="定位档案"
                        size="xs"
                        variant="outline"
                        @click="emit('locateArchive', archive.id)"
                      />
                    </div>
                  </div></div></template
            ></UPopover>
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
          {{ state.showOcr ? '收起 OCR 结果' : '查看 OCR 结果' }}
        </UButton>
        <div
          v-if="state.showOcr"
          class="rounded border border-default bg-elevated/50 p-3 text-sm break-words"
        >
          <p class="mb-1 text-xs text-muted">OCR 结果</p>
          {{ item.ocrResult || '未识别到文字' }}
        </div>
      </div>
    </div>
  </article>
</template>
