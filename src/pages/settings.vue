<script setup lang="ts">
import { oeaConfig, saving } from '@/utils/app/config';
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
              <USlider
                v-model="oeaConfig.soundVolume"
                class="flex-1"
                :max="1"
                :min="0"
                :step="0.05"
              />
              <span class="w-10 text-end text-sm tabular-nums">
                {{ Math.round(oeaConfig.soundVolume * 100) }}%
              </span>
            </div>
          </div>
        </UCard>
      </UPageBody>
    </UPage>
  </UContainer>
</template>
