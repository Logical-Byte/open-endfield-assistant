<script setup lang="ts">
import { computed } from 'vue';

const { state } = defineProps<{ state: string }>();

const isUnavailable = computed(() => state === 'load-error' || state === 'unsupported');
const statusTitle = computed(() => (state === 'unsupported' ? '桌面能力不可用' : '后端设置不可用'));
</script>

<template>
  <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_18rem]">
    <div class="space-y-4">
      <SettingsCard
        id="interface"
        class="scroll-mt-8"
        icon="i-lucide-layout-panel-left"
        title="界面设置"
      >
        <SettingsItem description="设置应用窗口的缩放比例" icon="i-lucide-zoom-in" title="缩放比例">
          <USlider
            v-if="state !== 'unsupported'"
            class="w-40"
            :max="2"
            :min="0.5"
            :model-value="1"
          />
          <span v-else class="text-sm text-muted">不可用</span>
        </SettingsItem>
        <SettingsItem
          description="点击关闭按钮时隐藏到系统托盘"
          icon="i-lucide-panel-bottom-close"
          title="关闭时最小化到托盘"
        >
          <USkeleton v-if="state === 'loading'" class="h-5 w-10 rounded-full" />
          <USwitch v-else-if="state === 'ready'" :model-value="true" />
          <span v-else class="text-sm text-muted">未读取</span>
        </SettingsItem>
        <SettingsItem
          description="进入档案扫描页时显示操作指引"
          icon="i-lucide-circle-help"
          title="显示新手操作提示"
        >
          <USkeleton v-if="state === 'loading'" class="h-5 w-10 rounded-full" />
          <USwitch v-else-if="state === 'ready'" :model-value="true" />
          <span v-else class="text-sm text-muted">未读取</span>
        </SettingsItem>
      </SettingsCard>

      <SettingsCard id="sound" class="scroll-mt-8" icon="i-lucide-headphones" title="声音设置">
        <SettingsItem
          description="扫描过程中的反馈音量"
          icon="i-lucide-volume-2"
          title="扫描提示音音量"
        >
          <USkeleton v-if="state === 'loading'" class="h-5 w-40" />
          <div v-else-if="state === 'ready'" class="flex w-40 gap-2">
            <USlider class="flex-1" :model-value="0.7" /><span class="text-sm">70%</span>
          </div>
          <span v-else class="text-sm text-muted">未读取</span>
        </SettingsItem>
      </SettingsCard>

      <SettingsCard id="update" class="scroll-mt-8" icon="i-lucide-download" title="更新设置">
        <SettingsItem
          v-for="item in ['更新源', '自动下载更新', '自动安装更新', '网络代理']"
          :key="item"
          :description="`${item}的应用设置`"
          icon="i-lucide-settings-2"
          :title="item"
        >
          <USkeleton v-if="state === 'loading'" class="h-6 w-24" />
          <UBadge v-else-if="state === 'ready'" color="success" label="已读取" variant="subtle" />
          <span v-else class="text-sm text-muted">未读取</span>
        </SettingsItem>
      </SettingsCard>

      <SettingsCard id="developer" class="scroll-mt-8" icon="i-lucide-code-2" title="开发者选项">
        <SettingsItem
          description="从给定的更新包运行一次原地更新流程"
          icon="i-lucide-package-open"
          title="安装更新包"
        >
          <UButton color="warning" label="选择更新包" variant="soft" />
        </SettingsItem>
      </SettingsCard>
    </div>

    <aside class="xl:sticky xl:top-4 xl:self-start">
      <UCard class="overflow-hidden rounded-xl" :ui="{ body: 'p-0 sm:p-0' }">
        <div class="border-b border-default p-4">
          <div class="mb-2 flex items-center gap-2">
            <UIcon
              :class="
                isUnavailable ? 'text-error' : state === 'ready' ? 'text-success' : 'text-primary'
              "
              :name="
                isUnavailable
                  ? 'i-lucide-circle-alert'
                  : state === 'ready'
                    ? 'i-lucide-circle-check'
                    : 'i-lucide-loader-circle'
              "
            />
            <p class="font-semibold">
              {{ isUnavailable ? statusTitle : state === 'ready' ? '设置已就绪' : '正在读取设置' }}
            </p>
          </div>
          <p class="text-sm text-dimmed">
            {{
              state === 'unsupported'
                ? '浏览器预览不会连接桌面后端。'
                : state === 'load-error'
                  ? '页面结构可用，具体值尚未读取。'
                  : state === 'ready'
                    ? '显示的是后端返回的完整设置。'
                    : '首次读取完成前不会填入默认值。'
            }}
          </p>
        </div>
        <div class="space-y-3 p-4 text-sm">
          <div class="flex justify-between">
            <span class="text-dimmed">配置来源</span
            ><span>{{ state === 'ready' ? 'Rust 后端' : '无' }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-dimmed">可编辑</span
            ><span>{{ state === 'ready' ? '是' : '否' }}</span>
          </div>
          <UButton
            v-if="state === 'load-error'"
            block
            icon="i-lucide-refresh-cw"
            label="重新加载"
          />
        </div>
      </UCard>
    </aside>
  </div>
</template>
