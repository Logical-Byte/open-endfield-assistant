<script setup lang="ts">
import type { ScanResult } from '@/lib/tauri';
import { openImagePreviewKey } from '@/utils/provideInject';
import { computed, inject } from 'vue';

const { index, status, category, image } = defineProps<ScanResult>();
const ocr_text = defineModel<string>('ocr_text');

/** 基准分辨率（16:9，实际分辨率不同时按此等比例缩放） */
const BASE_WIDTH = 1280;
const BASE_HEIGHT = 720;
/** 裁剪区域（以基准分辨率为坐标系） */
const CROP_LEFT = 343;
const CROP_TOP = 22;
const CROP_RIGHT = 936;
const CROP_BOTTOM = 168;
/** 裁剪区域尺寸 */
const CROP_WIDTH = CROP_RIGHT - CROP_LEFT;
const CROP_HEIGHT = CROP_BOTTOM - CROP_TOP;

/** 裁剪容器样式：保持裁剪区域的宽高比 */
const cropContainerStyle = computed(() => ({
  aspectRatio: `${CROP_WIDTH} / ${CROP_HEIGHT}`,
}));

/**
 * 裁剪图片样式：图片按基准分辨率等比放大（宽高比恒为 16:9），
 * 使裁剪区域恰好填满容器。
 * width/left 的 100% 指容器宽，height/top 的 100% 指容器高。
 */
const cropImageStyle = computed(() => ({
  width: `calc(100% * ${BASE_WIDTH} / ${CROP_WIDTH})`,
  height: `calc(100% * ${BASE_HEIGHT} / ${CROP_HEIGHT})`,
  left: `calc(100% * -${CROP_LEFT} / ${CROP_WIDTH})`,
  top: `calc(100% * -${CROP_TOP} / ${CROP_HEIGHT})`,
}));

const openImagePreview = inject(openImagePreviewKey, () => {
  console.warn('未提供 `openImagePreview` 方法');
});
</script>

<template>
  <UCard
    class="border-l-8"
    :class="{
      'border-error': status === 'failed',
      'border-success': status === 'success',
    }"
  >
    <div class="flex flex-col items-center gap-4 sm:flex-row">
      <div class="font-semibold">#{{ index }}</div>
      <UBadge color="neutral" :label="category" variant="outline" />

      <!-- <img alt="档案详情截图" class="h-32 self-start rounded-md" :src="image" /> -->
      <!-- B：仅显示 A 的裁剪区域（坐标与基准分辨率见脚本中的常量） -->
      <ImagePreviewContainer
        class="relative h-20 self-start overflow-hidden rounded-md ring-1 ring-default"
        :style="cropContainerStyle"
        @click="
          openImagePreview({
            url: image,
            name: `档案详情截图 #${index}`,
            downloadName: `档案详情截图 #${index}.png`,
          })
        "
      >
        <img
          alt="档案详情裁剪区域"
          class="absolute max-w-none"
          :src="image"
          :style="cropImageStyle"
        />
      </ImagePreviewContainer>
      <div class="flex min-w-0 flex-1 flex-col gap-2">
        <span class="text-xs font-medium text-muted">OCR 识别结果</span>
        <UTextarea
          v-model="ocr_text"
          autoresize
          color="neutral"
          :maxrows="8"
          placeholder="识别结果为空…"
          size="sm"
          variant="subtle"
        />
      </div>
    </div>
  </UCard>
</template>
