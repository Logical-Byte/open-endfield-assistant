<script setup lang="ts">
// 临时原型：比较原生按钮与输入、选择控件的 sm / xs 尺寸，不修改全局主题。
import { ref } from 'vue';
import WorkspaceSection from '@/components/layout/WorkspaceSection.vue';

type ButtonSize = 'xs' | 'sm' | 'md';
const selectedSize = ref<ButtonSize>('sm');
const sizes: ButtonSize[] = ['xs', 'sm', 'md'];
const isDevelopment = import.meta.env.DEV;
const controlSizes = ['sm', 'xs'] as const;
const sampleCategories = ['全部分类', '纸质记录', '电子档案'];
const sampleArchives = ['研究人员的笔记', '值班记录', '关于旧设施维护与人员交接的补充说明'];
</script>

<template>
  <div v-if="isDevelopment" class="space-y-2">
    <WorkspaceSection>
      <template #header>
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h1 class="font-semibold">常用控件：sm / xs 对齐比较</h1>
          <UBadge color="neutral" variant="subtle">临时原型</UBadge>
        </div>
      </template>
      <p class="mb-4 text-sm text-muted">
        可以输入、展开选项和搜索。所有控件沿用原生 padding、字体和图标。
      </p>
      <div class="grid grid-cols-[2rem_repeat(4,minmax(0,1fr))] items-center gap-3">
        <span />
        <span class="text-xs text-muted">按钮</span>
        <span class="text-xs text-muted">输入框</span>
        <span class="text-xs text-muted">普通下拉框</span>
        <span class="text-xs text-muted">可搜索下拉框</span>
        <template v-for="size in controlSizes" :key="size">
          <span class="text-xs text-muted">{{ size }}</span>
          <UButton
            class="justify-self-start"
            color="neutral"
            icon="i-lucide-list-filter"
            :size="size"
            variant="outline"
            >筛选</UButton
          >
          <UInput
            :aria-label="`输入框 ${size}`"
            class="w-full"
            default-value="研究人员的笔记"
            :size="size"
          />
          <USelect
            :aria-label="`普通下拉框 ${size}`"
            class="w-full"
            default-value="全部分类"
            :items="sampleCategories"
            :size="size"
          />
          <UInputMenu
            :aria-label="`可搜索下拉框 ${size}`"
            class="w-full"
            default-value="研究人员的笔记"
            :items="sampleArchives"
            :size="size"
          />
        </template>
      </div>
      <div class="mt-4 flex flex-wrap items-center gap-3 border-t border-default pt-3">
        <span class="text-xs text-muted">当前未指定 size 的搜索下拉框</span>
        <UInputMenu
          aria-label="搜索下拉框当前默认"
          class="w-56"
          default-value="研究人员的笔记"
          :items="sampleArchives"
        />
      </div>
    </WorkspaceSection>
    <WorkspaceSection>
      <template #header>
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h1 class="font-semibold">按钮试用：默认尺寸</h1>
          <UBadge color="neutral" variant="subtle">临时原型</UBadge>
        </div>
      </template>
      <p class="mb-3 text-sm text-muted">
        本轮只比较尺寸。所有示例使用 Nuxt UI 原生样式，按钮不会执行实际操作。
      </p>
      <div class="space-y-3">
        <div
          v-for="size in sizes"
          :key="size"
          class="flex flex-wrap items-center gap-3"
          :data-size-row="size"
        >
          <label class="flex w-32 shrink-0 items-center gap-2 text-sm whitespace-nowrap">
            <input v-model="selectedSize" name="button-size" type="radio" :value="size" />
            {{ size }}{{ size === 'md' ? '（库默认）' : '' }}
          </label>
          <UButton color="neutral" :size="size" variant="outline">普通按钮</UButton>
          <UButton icon="i-lucide-play" :size="size">开始扫描</UButton>
          <UButton
            color="neutral"
            :size="size"
            trailing-icon="i-lucide-chevron-down"
            variant="outline"
          >
            查看记录
          </UButton>
          <UButton
            aria-label="编辑示例"
            color="neutral"
            icon="i-lucide-pencil"
            :size="size"
            variant="outline"
          />
        </div>
      </div>
    </WorkspaceSection>

    <WorkspaceSection>
      <template #header>
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h2 class="font-semibold">使用场景</h2>
          <span aria-live="polite" class="text-sm text-muted">候选默认：{{ selectedSize }}</span>
        </div>
      </template>
      <div class="space-y-4" data-button-scenario>
        <div class="flex flex-wrap items-center gap-2">
          <UButton icon="i-lucide-play" :size="selectedSize">开始扫描</UButton>
          <UButton color="neutral" icon="i-lucide-download" :size="selectedSize" variant="outline"
            >导出</UButton
          >
          <UButton
            color="neutral"
            icon="i-lucide-list-filter"
            :size="selectedSize"
            variant="outline"
            >筛选</UButton
          >
          <UButton color="error" icon="i-lucide-trash-2" :size="selectedSize" variant="outline"
            >清空</UButton
          >
        </div>
        <div class="flex flex-wrap items-center gap-2 border-y border-default py-2">
          <div class="min-w-40 flex-1">
            <p class="text-sm">一份档案标题</p>
            <p class="text-xs text-muted">档案分类</p>
          </div>
          <UButton
            color="neutral"
            :size="selectedSize"
            trailing-icon="i-lucide-chevron-down"
            variant="outline"
            >查看记录</UButton
          >
          <UButton
            color="neutral"
            icon="i-lucide-external-link"
            :size="selectedSize"
            variant="outline"
            >查看攻略</UButton
          >
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <UButton color="neutral" :size="selectedSize" variant="outline">取消</UButton>
          <UButton icon="i-lucide-check" :size="selectedSize">确认</UButton>
          <UButton loading :size="selectedSize">处理中</UButton>
          <UButton disabled :size="selectedSize">暂不可用</UButton>
        </div>
      </div>
    </WorkspaceSection>
    <p class="px-3 pb-3 text-sm text-muted">
      选择仅影响本页示例。可以悬停、用 Tab 聚焦按钮，并通过侧栏切换明暗主题。
    </p>
  </div>
</template>
