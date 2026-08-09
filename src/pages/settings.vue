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
      </UPageBody>
    </UPage>
  </UContainer>
</template>
