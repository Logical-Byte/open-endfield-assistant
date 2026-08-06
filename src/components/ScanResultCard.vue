<script setup lang="ts">
import { categoryName, initPrtsData, pageName, titlesOfCategory } from '@/lib/prtsData';
import type { ScanResult } from '@/lib/tauri';
import { openImagePreviewKey } from '@/utils/provideInject';
import { computed, inject } from 'vue';

const { index, status, category, sub_category, image, corrected_title, item_ids } =
  defineProps<ScanResult>();
const ocr_text = defineModel<string>('ocr_text');

// 自动补全候选：当前子分类下所有档案标题（prts 数据加载幂等）
initPrtsData();
const candidates = computed(() => titlesOfCategory(sub_category));

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
      'border-warning': status === 'unrecognized',
      'border-success': status === 'success',
    }"
  >
    <div class="flex flex-col items-center gap-4 sm:flex-row">
      <div class="flex flex-col items-center gap-1">
        <div class="font-semibold">#{{ index }}</div>
        <UBadge v-if="category" color="neutral" :label="pageName(category)" variant="outline" />
        <UBadge
          v-if="sub_category"
          color="secondary"
          :label="categoryName(sub_category)"
          variant="outline"
        />
        <UBadge v-if="corrected_title" color="success" label="已纠错" variant="soft" />
        <UBadge
          v-else-if="status === 'unrecognized'"
          color="warning"
          label="无法识别"
          variant="soft"
        />
      </div>

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
        <span class="text-xs font-medium text-muted">档案标题</span>
        <UInputMenu
          v-model="ocr_text"
          color="neutral"
          :content="{ hideWhenEmpty: true }"
          :items="candidates"
          mode="autocomplete"
          placeholder="识别结果为空…"
          size="sm"
          variant="subtle"
        />
        <p v-if="item_ids.length" class="text-xs text-muted">档案 ID：{{ item_ids.join('、') }}</p>
      </div>
    </div>
  </UCard>
</template>
