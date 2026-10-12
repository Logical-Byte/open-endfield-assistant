<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { editSettings, settingsState } from '@/features/settings/settings';
import { computed, ref } from 'vue';

/**
 * 修改下方扫描指引文案时，如果要求所有用户重新查看，应递增 `settingsStore.ts` 的
 * `CURRENT_SCAN_TIPS_VERSION`。
 */

/** 本次启动内已手动关闭（未勾选持久化时仅隐藏本次启动）。 */
const { t } = useAppI18n();

const dismissedThisSession = ref(false);

/**
 * 是否显示启动扫描提示。
 * 设置初始化完成前不渲染，避免启动时短暂闪现提示。
 * 初始化完成后，`settingsState.value.effective.scanGuideEnabled` 为 `true`
 * 且本次启动内未手动关闭时显示。提示版本的比较和编码由 `settingsStore.ts` 负责。
 */
const showScanGuide = computed(
  () =>
    settingsState.value.status === 'ready' &&
    settingsState.value.effective.scanGuideEnabled &&
    !dismissedThisSession.value,
);

/** 是否勾选「下次更新前不再提示」。勾选后点击「我知道了」会将确认版本写入设置并持久化。 */
const dismissGuide = ref(false);

/**
 * 关闭提示。
 *
 * 勾选时提交关闭提示，版本编码和保存由设置模块负责。
 * 保存成功后隐藏。保存失败则保留提示，让用户可以重试。
 * 未勾选：仅本次启动内隐藏，不写设置，下次启动仍会展示。
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
        <span>{{ t('scan.guide.open') }}</span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">2.</span>
        <span>{{ t('scan.guide.hdr') }}</span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">3.</span>
        <span>{{ t('scan.guide.archive') }}</span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">4.</span>
        <span>{{ t('scan.guide.scan') }}</span>
      </li>
      <li class="flex items-baseline gap-4">
        <span class="w-8 flex-none text-right text-primary">5.</span>
        <span>{{ t('scan.guide.export') }}</span>
      </li>
    </ol>

    <div class="flex flex-col items-center gap-4">
      <UCheckbox v-model="dismissGuide" :label="t('scan.hideGuide')" />
      <UButton :label="t('scan.gotIt')" size="lg" @click="dismissScanGuide" />
    </div>
  </div>
</template>
