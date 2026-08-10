<script setup lang="ts">
import { oeaConfig, saving } from '@/utils/app/config';
import { computed } from 'vue';

/**
 * 音量（本地数字中转）。
 *
 * Nuxt UI 的 USlider 内部把 v-model 当数组处理，拖动时可能先回写 `[v]`（数组）再回写 `v`（数字），
 * 直接绑到 oeaConfig.soundVolume 会让数组短暂进入配置，触发 save_oea_config 反序列化失败
 * （"invalid type: sequence, expected f32"）。这里只允许 number 写入配置，数组一律忽略。
 */
const soundVolume = computed<number>({
  get() {
    return oeaConfig.value.soundVolume;
  },
  set(value: number) {
    // 只在 value 是数字时写入配置，数组一律忽略
    if (typeof value === 'number') {
      oeaConfig.value.soundVolume = value;
    }
  },
});
</script>

<template>
  <UContainer>
    <UPage>
      <UPageBody>
        <UCard description="托盘图标与窗口行为" title="系统托盘">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
              <span class="i-lucide-panel-bottom-dashed text-2xl text-primary" />
              <div>
                <p class="font-medium">关闭时最小化到托盘</p>
                <p class="text-sm text-toned">
                  点击窗口关闭按钮时隐藏到系统托盘而不是退出，可通过托盘菜单或 Alt+Delete 退出
                </p>
              </div>
            </div>
            <USwitch v-model="oeaConfig.minimizeToTray" :loading="saving" />
          </div>
        </UCard>

        <UCard description="扫描开始、完成与失败提示音" title="扫描音效">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
              <span class="i-lucide-volume-2 text-2xl text-primary" />
              <div>
                <p class="font-medium">扫描提示音音量</p>
                <p class="text-sm text-toned">
                  扫描开始与自然完成时播放提示音，失败或被停止时播放另一提示音
                </p>
              </div>
            </div>
            <div class="flex w-52 items-center gap-3">
              <USlider v-model="soundVolume" class="flex-1" :max="1" :min="0" :step="0.05" />
              <span class="w-10 text-end text-sm tabular-nums">
                {{ Math.round(soundVolume * 100) }}%
              </span>
            </div>
          </div>
        </UCard>
      </UPageBody>
    </UPage>
  </UContainer>
</template>
