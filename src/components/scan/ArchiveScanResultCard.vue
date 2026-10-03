<script setup lang="ts">
import type { ArchiveScanResultCardProps } from '@/features/archiveScan/types/archiveScanResultCard';
import { ArchiveScanCardStatus } from '@/features/archiveScan/types/archiveScanResultCard';
import { openImagePreview } from '@/composables/image-preview';
import type { CSSProperties } from 'vue';

const {
  status,
  categoryLabel,
  imageUrl,
  title,
  candidates,
  acquisitionMethod,
  acquisitionLabel,
  oemUrl,
  intelUrl,
} = defineProps<ArchiveScanResultCardProps>();

const emit: (event: 'correct', title: string) => void = defineEmits<{
  correct: [title: string];
}>();

/** 基准分辨率（16:9，实际分辨率不同时按此等比例缩放） */
const BASE_WIDTH: number = 1280;
const BASE_HEIGHT: number = 720;
/** 裁剪区域（以基准分辨率为坐标系） */
const CROP_LEFT: number = 360;
const CROP_TOP: number = 48;
const CROP_RIGHT: number = 876;
const CROP_BOTTOM: number = 134;
/** 裁剪区域尺寸 */
const CROP_WIDTH: number = CROP_RIGHT - CROP_LEFT;
const CROP_HEIGHT: number = CROP_BOTTOM - CROP_TOP;

/** 裁剪容器样式：保持裁剪区域的宽高比 */
const cropContainerStyle: CSSProperties = {
  aspectRatio: `${CROP_WIDTH} / ${CROP_HEIGHT}`,
};

/**
 * 裁剪图片样式：图片按基准分辨率等比放大（宽高比恒为 16:9），
 * 使裁剪区域恰好填满容器。
 * width/left 的 100% 指容器宽，height/top 的 100% 指容器高。
 */
const cropImageStyle: CSSProperties = {
  width: `calc(100% * ${BASE_WIDTH} / ${CROP_WIDTH})`,
  height: `calc(100% * ${BASE_HEIGHT} / ${CROP_HEIGHT})`,
  left: `calc(100% * -${CROP_LEFT} / ${CROP_WIDTH})`,
  top: `calc(100% * -${CROP_TOP} / ${CROP_HEIGHT})`,
};
</script>

<template>
  <UCard
    class="border-l-8"
    :class="{
      'border-success': status === ArchiveScanCardStatus.Matched,
      'border-warning': status === ArchiveScanCardStatus.Unrecognized,
      'border-error': status === ArchiveScanCardStatus.OcrFailed,
      'border-(--ui-text-dimmed)': status === ArchiveScanCardStatus.NotMatched,
    }"
    :ui="{
      body: 'px-3! py-0!',
    }"
  >
    <div class="flex flex-col items-center gap-3 sm:flex-row">
      <div class="flex w-31.5 items-center justify-center">
        <UBadge v-if="categoryLabel" color="info" :label="categoryLabel" variant="outline" />
      </div>

      <div class="relative w-72">
        <ImagePreviewContainer
          v-if="imageUrl"
          class="relative w-full overflow-hidden"
          :class="{
            'opacity-25': status === ArchiveScanCardStatus.Matched,
          }"
          :style="cropContainerStyle"
          @click="
            openImagePreview({
              url: imageUrl,
              name: `档案详情截图`,
              downloadName: (): string => `档案详情截图 - ${title}.png`,
            })
          "
        >
          <img
            alt="档案详情截图"
            class="absolute max-w-none"
            :src="imageUrl"
            :style="cropImageStyle"
          />
        </ImagePreviewContainer>
        <div v-else class="flex h-12 items-center justify-center bg-accented" />
        <!-- 图片预览右下角状态标记：已收集 / 需纠错 -->
        <UBadge
          v-if="status === ArchiveScanCardStatus.Matched"
          class="absolute right-1 bottom-1"
          color="success"
          label="已收集"
          leading-icon="i-lucide-check"
          size="sm"
        />
        <UBadge
          v-else-if="
            status === ArchiveScanCardStatus.Unrecognized ||
            status === ArchiveScanCardStatus.OcrFailed
          "
          class="absolute right-1 bottom-1"
          color="error"
          label="需纠错"
          leading-icon="i-lucide-pencil-line"
          size="sm"
        />
        <UBadge
          v-else-if="status === ArchiveScanCardStatus.NotMatched"
          class="absolute right-1 bottom-1"
          color="neutral"
          label="未收集"
          leading-icon="i-lucide-eye"
          size="sm"
        />
      </div>

      <div class="min-w-0 flex-1">
        <p v-if="status === ArchiveScanCardStatus.NotMatched" class="text-center">{{ title }}</p>
        <div v-else class="flex flex-col">
          <!-- <p class="text-xs font-medium text-muted">标题识别纠错</p> -->
          <UInputMenu
            color="neutral"
            :items="candidates"
            mode="autocomplete"
            :model-value="title ?? ''"
            placeholder="选择或输入档案标题"
            @update:model-value="(value: string) => emit('correct', value)"
          />
        </div>
      </div>

      <div class="flex w-36 justify-center">
        <UButton
          v-if="acquisitionMethod === 'map' && status === ArchiveScanCardStatus.Matched"
          class="text-muted"
          color="neutral"
          label="在 OEM 中查看"
          target="_blank"
          :to="oemUrl ?? undefined"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
        <UButton
          v-else-if="acquisitionMethod === 'map' && status === ArchiveScanCardStatus.NotMatched"
          label="前往 OEM 收集"
          target="_blank"
          :to="oemUrl ?? undefined"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
        <UButton
          v-else-if="acquisitionLabel"
          :class="{ 'text-muted': status === ArchiveScanCardStatus.Matched }"
          color="neutral"
          :label="acquisitionLabel"
          size="sm"
          target="_blank"
          :to="intelUrl ?? undefined"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
      </div>
    </div>
  </UCard>
</template>
