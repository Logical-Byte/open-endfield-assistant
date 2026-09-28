<script setup lang="ts">
import { configInitialized, editSettings, effectiveSettings } from '@/utils/app/config';
import { computed, ref } from 'vue';

/**
 * 修改下方扫描指引文案时，如果要求所有用户重新查看，应递增 `configStore.ts` 的
 * `CURRENT_SCAN_TIPS_VERSION`。
 */

/** 本次启动内已手动关闭（未勾选持久化时仅隐藏本次启动）。 */
const dismissedThisSession = ref(false);

/**
 * 是否显示启动扫描提示。
 * 配置加载完成前不渲染，避免启动时用默认配置短暂闪现提示。
 * 加载完成后，`effectiveSettings.value.scanGuideEnabled` 为 `true`
 * 且本次启动内未手动关闭时显示。提示版本的比较和编码由 `configStore.ts` 负责。
 */
const showScanGuide = computed(
  () =>
    configInitialized.value &&
    effectiveSettings.value.scanGuideEnabled &&
    !dismissedThisSession.value,
);

/** 是否勾选「下次更新前不再提示」。勾选后点击「我知道了」会将确认版本写入配置并持久化。 */
const dismissGuide = ref(false);

/**
 * 关闭提示。
 *
 * 勾选时提交关闭提示，版本编码和保存由设置模块负责。
 * 保存成功后隐藏；保存失败则保留提示，让用户可以重试。
 * 未勾选：仅本次启动内隐藏，不写配置，下次启动仍会展示。
 */
function dismissScanGuide(): void {
  if (dismissGuide.value) {
    editSettings({ scanGuideEnabled: false });
  } else {
    dismissedThisSession.value = true;
  }
}
</script>

<template>
  <div
    v-if="showScanGuide"
    class="absolute inset-0 z-20 flex flex-col items-center justify-center gap-10 bg-default px-6 text-center"
  >
    <ol class="inline-flex flex-col gap-5 text-left text-2xl leading-relaxed font-semibold">
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">1.</span>
        <span>
          打开终末地，调成 <strong class="text-primary">1280 × 720</strong>、<strong
            class="text-primary"
            >简体中文</strong
          >
        </span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">2.</span>
        <span><strong class="text-primary">关闭 HDR</strong>，关闭性能监控软件</span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">3.</span>
        <span>终末地打开<strong class="text-primary">档案库界面</strong></span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">4.</span>
        <span>点击左上角<strong class="text-primary">开始扫描</strong></span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">5.</span>
        <span>扫完点击右上角<strong class="text-primary">导出到地图集</strong></span>
      </li>
    </ol>

    <div class="flex flex-col items-center gap-4">
      <UCheckbox v-model="dismissGuide" label="下次更新前不再提示" />
      <UButton label="我知道了" size="lg" @click="dismissScanGuide" />
    </div>
  </div>
</template>
