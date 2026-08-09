<script setup lang="ts">
import type { AcquisitionMethod } from '@/types/archiveAcquisitionContract';
import type { ScanResultCardProps } from '@/types/scanResult';
import { CollectType } from '@/types/scanResult';
import { getAcquisitionMethod } from '@/utils/app/archiveAcquisitionContract';
import { openImagePreviewKey } from '@/utils/provideInject';
import { getCategoryName, getCategoryTitles, getPageName } from '@/utils/prts';
import { computed, inject } from 'vue';

const { collectType, category, subCategory, imageUrl, title, archiveId } =
  defineProps<ScanResultCardProps>();

// 自动补全候选：当前子分类下所有档案标题（prts 数据加载幂等）
const candidates = computed(() => getCategoryTitles(subCategory));

/** 非地图获取方式的展示文本（仅地图交互点位显示 OEM 按钮） */
const ACQUISITION_METHOD_LABELS: Record<Exclude<AcquisitionMethod, 'map'>, string> = {
  mission: '完成任务获取',
  spec: '特殊交互获取',
  auto: '自动解锁',
  shop: '商店兑换获取',
  invstgt: '研究提交解锁',
};

/** 当前档案的获取方式（未知 / 未收录时为 null） */
const acquisitionMethod = computed(() => (archiveId ? getAcquisitionMethod(archiveId) : null));

/** 是否为地图交互点位（仅该类显示 OEM 按钮） */
const isMapPoint = computed(() => acquisitionMethod.value === 'map');

/** 非地图获取方式的展示文本（地图点位 / 未知时不显示） */
const acquisitionLabel = computed(() => {
  const method = acquisitionMethod.value;
  return method && method !== 'map' ? ACQUISITION_METHOD_LABELS[method] : null;
});

/** OEM 档案链接（地图交互点位，指向 https://oem.re/?type=<档案 id>） */
const oemUrl = computed(() => (archiveId ? `https://oem.re/?type=${archiveId}` : null));

/** 基准分辨率（16:9，实际分辨率不同时按此等比例缩放） */
const BASE_WIDTH = 1280;
const BASE_HEIGHT = 720;
/** 裁剪区域（以基准分辨率为坐标系） */
const CROP_LEFT = 360;
const CROP_TOP = 48;
const CROP_RIGHT = 876;
const CROP_BOTTOM = 134;
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
      'border-success': collectType === CollectType.Collected,
      'border-warning': collectType === CollectType.Unrecognized,
      'border-error': collectType === CollectType.Failed,
      'border-(--ui-text-dimmed)': collectType === CollectType.NotCollected,
    }"
    :ui="{
      body: 'px-3! py-0!',
    }"
  >
    <div class="flex flex-col items-center gap-3 sm:flex-row">
      <div class="flex w-31.5 items-center justify-center">
        <UBadge
          v-if="category && subCategory"
          color="info"
          :label="`${getPageName(category)} − ${getCategoryName(subCategory)}`"
          variant="outline"
        />
      </div>

      <div class="w-72">
        <ImagePreviewContainer
          v-if="imageUrl"
          class="relative w-full overflow-hidden"
          :class="{
            'opacity-25': collectType === CollectType.Collected,
          }"
          :style="cropContainerStyle"
          @click="
            openImagePreview({
              url: imageUrl,
              name: `档案详情截图`,
              downloadName: () => `档案详情截图 - ${title}.png`,
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
        <div v-else class="flex h-12 items-center justify-center bg-accented">
          <p class="text-sm text-muted">待收集</p>
        </div>
      </div>

      <div class="min-w-0 flex-1">
        <p v-if="collectType === CollectType.NotCollected" class="text-center">{{ title }}</p>
        <div v-else class="flex flex-col">
          <!-- <p class="text-xs font-medium text-muted">标题识别纠错</p> -->
          <UInputMenu color="neutral" :items="candidates" :model-value="title ?? ''" />
        </div>
      </div>

      <div class="flex w-36 justify-center">
        <UButton
          v-if="isMapPoint && collectType === CollectType.Collected"
          class="text-muted"
          color="neutral"
          label="在 OEM 中查看"
          target="_blank"
          :to="oemUrl ?? undefined"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
        <UButton
          v-else-if="isMapPoint && collectType === CollectType.NotCollected"
          label="前往 OEM 收集"
          target="_blank"
          :to="oemUrl ?? undefined"
          trailing-icon="i-lucide-external-link"
          variant="outline"
        />
        <UBadge
          v-else-if="acquisitionLabel"
          color="neutral"
          :label="acquisitionLabel"
          variant="outline"
        />
      </div>
    </div>
  </UCard>
</template>
