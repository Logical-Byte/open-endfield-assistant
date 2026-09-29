<script setup lang="ts">
const { state } = defineProps<{ state: string }>();

const groups = [
  {
    id: 'interface',
    title: '界面设置',
    icon: 'i-lucide-layout-panel-left',
    items: ['缩放比例', '关闭时最小化到托盘', '显示新手操作提示'],
  },
  { id: 'sound', title: '声音设置', icon: 'i-lucide-headphones', items: ['扫描提示音音量'] },
  {
    id: 'update',
    title: '更新设置',
    icon: 'i-lucide-download',
    items: ['更新源', '自动下载更新', '自动安装更新', 'Mirror酱 CDK', '网络代理'],
  },
  { id: 'developer', title: '开发者选项', icon: 'i-lucide-code-2', items: ['安装更新包'] },
];

function itemAvailable(groupId: string, item: string): boolean {
  if (groupId === 'developer') return state !== 'unsupported';
  if (item === '缩放比例') return state !== 'unsupported';
  return state === 'ready';
}
</script>

<template>
  <div
    v-if="state === 'load-error' || state === 'unsupported'"
    class="rounded-2xl border border-error/25 bg-error/5 p-5"
  >
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-start gap-3">
        <div class="rounded-full bg-error/10 p-2 text-error">
          <UIcon
            class="text-xl"
            :name="state === 'unsupported' ? 'i-lucide-circle-slash-2' : 'i-lucide-circle-alert'"
          />
        </div>
        <div>
          <h2 class="font-semibold">
            {{ state === 'unsupported' ? '桌面设置在浏览器中不可用' : '暂时无法连接设置服务' }}
          </h2>
          <p class="mt-1 text-sm text-dimmed">设置目录仍然可见，未读取的值不会以默认值填充。</p>
        </div>
      </div>
      <UButton v-if="state === 'load-error'" icon="i-lucide-refresh-cw" label="重新连接" />
    </div>
  </div>

  <div class="grid gap-4 lg:grid-cols-2">
    <UCard
      v-for="group in groups"
      :id="group.id"
      :key="group.id"
      class="scroll-mt-8 rounded-2xl"
      :ui="{ body: 'p-0 sm:p-0' }"
    >
      <template #header
        ><div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <UIcon class="text-primary" :name="group.icon" /><span class="font-semibold">{{
              group.title
            }}</span>
          </div>
          <UBadge color="neutral" :label="`${group.items.length} 项`" variant="subtle" /></div
      ></template>
      <div class="divide-y divide-default">
        <div
          v-for="item in group.items"
          :key="item"
          class="flex min-h-14 items-center justify-between gap-4 px-4 py-3"
        >
          <span class="text-sm font-medium">{{ item }}</span>
          <USkeleton
            v-if="state === 'loading' && item !== '缩放比例' && group.id !== 'developer'"
            class="h-5 w-20"
          />
          <UBadge
            v-else-if="itemAvailable(group.id, item)"
            color="success"
            icon="i-lucide-check"
            :label="state === 'ready' ? '已读取' : '独立可用'"
            variant="subtle"
          />
          <div v-else class="flex items-center gap-2 text-xs text-muted">
            <span>不可用</span><UIcon name="i-lucide-lock-keyhole" />
          </div>
        </div>
      </div>
    </UCard>
  </div>
</template>
