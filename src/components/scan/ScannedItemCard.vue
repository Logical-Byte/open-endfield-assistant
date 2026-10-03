<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useEvidencePopover } from './useEvidencePopover';
import type { ScannedItemView } from '@/features/archiveScan/resultView';
import type { ArchiveId } from '@/features/archiveScan/types/scannedItem';
import { openImagePreview } from '@/composables/image-preview';
import type { CSSProperties, Ref } from 'vue';
const props = defineProps<{
  item: ScannedItemView;
  selected?: boolean;
}>();
const emit = defineEmits<{
  correct: [title: string];
  locateArchive: [id: ArchiveId];
  clearHighlight: [];
}>();
const draft: Ref<string> = ref(props.item.correctedTitle ?? props.item.ocrResult);
const editing: Ref<boolean> = ref(false);
const {
  open: floatingMatchesOpen,
  cancelClose,
  scheduleClose,
} = useEvidencePopover((): void => emit('clearHighlight'));

watch(
  () => props.item.correctedTitle,
  (): void => {
    draft.value = props.item.correctedTitle ?? props.item.ocrResult;
  },
);
const options = computed((): string[] => [...props.item.candidates]);
function submit(): void {
  emit('correct', draft.value);
  editing.value = false;
}
// 与原扫描卡片一致，缩略图裁剪标题区域，点击查看完整截图。
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
    class="scan-card relative isolate overflow-hidden rounded-lg border px-3 py-2.5 transition-colors"
    :class="[
      selected ? 'border-primary bg-primary/5' : 'border-default bg-default',
      'gradient-surface',
    ]"
    :style="{
      '--matching-tint': item.archives.length
        ? 'var(--matching-matched-background)'
        : 'var(--matching-unknown-background)',
    }"
  >
    <div>
      <div class="scan-heading mb-2 flex flex-wrap items-center gap-x-2 gap-y-1">
        <UBadge :color="item.archives.length ? 'success' : 'neutral'" size="sm" variant="soft">{{
          item.archives.length ? '有匹配' : '未匹配'
        }}</UBadge>
        <span class="font-mono text-xs text-muted"
          >#{{ String(item.scannedItemId).padStart(3, '0') }}</span
        ><span class="text-xs text-muted">{{ item.categoryLabel }}</span
        ><span
          v-if="!item.archives.length"
          class="text-xs"
          :class="item.status === 'failed' ? 'text-error' : 'text-warning'"
          >{{ item.status === 'failed' ? 'OCR 未识别到文字' : '标题未命中目录' }}</span
        >
        <UButton
          v-if="item.archives.length && !editing"
          class="ml-auto shrink-0"
          color="neutral"
          icon="i-lucide-pencil"
          label="修改"
          size="xs"
          variant="ghost"
          @click="editing = true"
        />
      </div>
      <div class="scan-layout scan-left">
        <div class="scan-crop flex items-center rounded">
          <ImagePreviewContainer
            class="relative w-full overflow-hidden rounded"
            :style="cropContainerStyle"
            @click="
              openImagePreview({
                url: item.image,
                name: '档案详情截图',
                downloadName: `档案详情截图 - ${item.correctedTitle ?? item.ocrResult}.png`,
              })
            "
          >
            <img
              alt="档案详情截图"
              class="absolute max-w-none"
              :src="item.image"
              :style="cropImageStyle"
            />
          </ImagePreviewContainer>
        </div>
        <div class="min-w-0 flex-1">
          <p class="text-xs leading-5 break-words text-muted" :title="item.ocrResult">
            OCR：{{ item.ocrResult || '无文字' }}
          </p>
          <form
            v-if="!item.archives.length || editing"
            class="mt-1 flex items-center gap-1.5"
            @submit.prevent="submit"
          >
            <UInputMenu
              v-model="draft"
              aria-label="正确档案标题"
              class="min-w-0 flex-1"
              :items="options"
              mode="autocomplete"
              placeholder="正确标题"
              size="sm"
            /><UButton
              class="shrink-0"
              icon="i-lucide-check"
              label="确认"
              size="sm"
              type="submit"
            />
          </form>
          <div v-else class="mt-0.5 flex items-center justify-between gap-1">
            <span
              class="min-w-0 flex-1 text-sm font-medium break-words"
              :title="item.correctedTitle ?? item.ocrResult"
              >{{ item.correctedTitle ?? item.ocrResult }}</span
            >
          </div>
          <div v-if="item.archives.length" class="mt-1 flex items-center justify-between gap-1">
            <span class="text-[11px] text-muted">{{
              item.manuallyCorrected ? '人工纠正' : '自动匹配'
            }}</span>
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
                variant="ghost"
                @mouseenter="cancelClose"
                @mouseleave="scheduleClose" /><template #content
                ><div @mouseenter="cancelClose" @mouseleave="scheduleClose">
                  <p class="mb-2 text-xs text-muted">
                    扫描 #{{ item.scannedItemId }} · 关联 {{ item.archives.length }} 份档案
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
                        variant="ghost"
                        @click="emit('locateArchive', archive.id)"
                      />
                    </div>
                  </div></div></template
            ></UPopover>
          </div>
        </div>
      </div>
      <p v-if="item.manuallyCorrected && !item.archives.length" class="mt-1.5 text-xs text-warning">
        输入的标题未命中当前分类，请重新选择。
      </p>
    </div>
  </article>
</template>

<style scoped>
.scan-layout {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
.scan-crop {
  width: 128px;
  flex-shrink: 0;
  min-height: 72px;
  background: var(--ui-bg-accented);
}
.gradient-surface {
  background-image: linear-gradient(
    90deg,
    color-mix(in srgb, var(--matching-tint) 7%, transparent) 12%,
    transparent 45%
  );
}
@media (max-width: 847px) {
  .scan-left {
    flex-direction: column;
    align-items: stretch;
  }
  .scan-left .scan-crop {
    width: auto;
  }
}
</style>
