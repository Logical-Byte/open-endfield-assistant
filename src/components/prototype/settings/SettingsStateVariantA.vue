<script setup lang="ts">
import { computed } from 'vue';

const { state } = defineProps<{ state: string }>();

const unavailableLabel = computed(() =>
  state === 'unsupported' ? '需要在桌面应用中使用' : '设置暂不可用',
);
</script>

<template>
  <UAlert
    v-if="state === 'load-error'"
    :actions="[{ label: '重新加载', icon: 'i-lucide-refresh-cw' }]"
    color="error"
    description="无法从后端读取设置。界面缩放和开发者功能仍可使用。"
    icon="i-lucide-circle-alert"
    title="设置加载失败"
    variant="subtle"
  />
  <UAlert
    v-else-if="state === 'unsupported'"
    color="neutral"
    description="纯浏览器模式只用于预览页面壳。请运行桌面开发模式来读取和修改设置。"
    icon="i-lucide-monitor-off"
    title="浏览器模式不支持应用设置"
    variant="subtle"
  />

  <SettingsCard
    id="interface"
    class="scroll-mt-8"
    icon="i-lucide-layout-panel-left"
    title="界面设置"
  >
    <SettingsItem
      description="设置应用窗口的缩放比例，影响所有界面元素的大小"
      icon="i-lucide-zoom-in"
      title="缩放比例"
    >
      <div v-if="state !== 'unsupported'" class="flex w-56 items-center gap-2">
        <USlider class="flex-1" :max="2" :min="0.5" :model-value="1" :step="0.05" />
        <span class="w-12 text-end text-sm tabular-nums">100%</span>
      </div>
      <UBadge
        v-else
        color="neutral"
        icon="i-lucide-monitor-off"
        label="浏览器中不可用"
        variant="subtle"
      />
    </SettingsItem>
    <SettingsItem
      description="点击窗口关闭按钮时隐藏到系统托盘而不是退出"
      icon="i-lucide-panel-bottom-close"
      title="关闭时最小化到托盘"
    >
      <USkeleton v-if="state === 'loading'" class="h-5 w-10 rounded-full" />
      <USwitch v-else-if="state === 'ready'" :model-value="true" />
      <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
    </SettingsItem>
    <SettingsItem
      description="进入档案扫描页时显示操作指引"
      icon="i-lucide-circle-help"
      title="显示新手操作提示"
    >
      <USkeleton v-if="state === 'loading'" class="h-5 w-10 rounded-full" />
      <USwitch v-else-if="state === 'ready'" :model-value="true" />
      <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
    </SettingsItem>
  </SettingsCard>

  <SettingsCard id="sound" class="scroll-mt-8" icon="i-lucide-headphones" title="声音设置">
    <SettingsItem
      description="扫描开始、完成、失败或被停止时播放提示音"
      icon="i-lucide-volume-2"
      title="扫描提示音音量"
    >
      <USkeleton v-if="state === 'loading'" class="h-5 w-56" />
      <div v-else-if="state === 'ready'" class="flex w-56 items-center gap-2">
        <USlider class="flex-1" :model-value="0.7" /><span class="text-sm">70%</span>
      </div>
      <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
    </SettingsItem>
  </SettingsCard>

  <SettingsCard id="update" class="scroll-mt-8" icon="i-lucide-download" title="更新设置">
    <SettingsItem
      v-for="item in [
        ['更新源', '选择从哪个源下载新版本', 'i-lucide-cloud-download'],
        ['自动下载更新', '检查到新版本后自动开始下载', 'i-lucide-download-cloud'],
        ['自动安装更新', '更新包下载完成后自动安装', 'i-lucide-rocket'],
        ['网络代理', '下载更新包时使用的代理方式', 'i-lucide-network'],
      ]"
      :key="item[0]"
      :description="item[1]"
      :icon="item[2]"
      :title="item[0]"
    >
      <USkeleton v-if="state === 'loading'" class="h-8 w-36" />
      <template v-else-if="state === 'ready'">
        <USelect
          v-if="item[0] === '更新源' || item[0] === '网络代理'"
          class="w-36"
          :items="['GitHub', '自动选择']"
          model-value="GitHub"
        />
        <USwitch v-else :model-value="true" />
      </template>
      <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
    </SettingsItem>
  </SettingsCard>

  <SettingsCard id="developer" class="scroll-mt-8" icon="i-lucide-code-2" title="开发者选项">
    <SettingsItem
      description="从给定的更新包运行一次原地更新流程"
      icon="i-lucide-package-open"
      title="安装更新包"
    >
      <UButton color="warning" icon="i-lucide-package-open" label="选择更新包" variant="soft" />
    </SettingsItem>
  </SettingsCard>
</template>
