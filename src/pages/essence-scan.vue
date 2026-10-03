<script setup lang="ts">
import { computed, ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import EssenceResultCard from '@/components/essenceScan/EssenceResultCard.vue';
import EssenceScanRules from '@/components/essenceScan/EssenceScanRules.vue';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import {
  catalog,
  catalogError,
  catalogStatus,
  initEssenceCatalog,
} from '@/features/essenceScan/catalog';
import { clearScannedItems, scannedItems, scanError } from '@/features/essenceScan/scannedItems';
import {
  decisionPresentation,
  deriveSummary,
  filterItems,
} from '@/features/essenceScan/resultView';
import type { Decision } from '@/features/essenceScan/types';
import { essenceScanWorkerType } from '@/features/essenceScan/workerType';
import { initOeaSettings, retrySettingsSave, settingsState } from '@/features/settings/settings';

const desktop = isTauri();
const task = useAutomationTask('essenceScan');
const { phase, isActive, canStart, canStop, outcome } = task;
const commandBusy = ref(false);
const commandError = ref<string | null>(null);
const query = ref('');
const decision = ref<'all' | Decision>('all');
const decisions: Decision[] = ['keep', 'discard', 'skip', 'review'];
const decisionOptions: { value: 'all' | Decision; label: string }[] = [
  { value: 'all', label: '全部判断' },
  ...decisions.map((value) => ({ value, label: decisionPresentation[value].label })),
];
const summary = computed(() => deriveSummary(scannedItems.value));
const filteredItems = computed(() =>
  filterItems(scannedItems.value, catalog.value, { decision: decision.value, query: query.value }),
);
const readySettings = computed(() =>
  settingsState.value.status === 'ready' ? settingsState.value : null,
);
const rulesUnsaved = computed(
  (): boolean =>
    readySettings.value !== null &&
    readySettings.value.draft.essenceScan !== readySettings.value.effective.essenceScan,
);
const settingsError = computed((): string | null =>
  settingsState.value.status === 'unavailable' &&
  settingsState.value.reason.type === 'initialize-error'
    ? settingsState.value.reason.error.message
    : null,
);
const canBegin = computed(
  (): boolean =>
    desktop &&
    canStart.value &&
    readySettings.value !== null &&
    !rulesUnsaved.value &&
    catalogStatus.value === 'ready',
);
const buttonDisabled = computed(
  (): boolean => commandBusy.value || (!canBegin.value && !canStop.value),
);
const buttonLabel = computed((): string => {
  if (phase.value === 'stopping') return '正在停止扫描';
  if (phase.value === 'blocked') return '其他自动化任务正在运行';
  if (phase.value === 'running') return '停止扫描';
  if (desktop && settingsState.value.status === 'initializing') return '正在加载设置';
  if (desktop && (catalogStatus.value === 'idle' || catalogStatus.value === 'loading'))
    return '正在加载目录';
  if (rulesUnsaved.value) return readySettings.value?.saveError ? '规则尚未保存' : '正在保存规则';
  return essenceScanWorkerType.value === 'simulation' ? '开始模拟扫描' : '开始基质扫描';
});
const statusLabel = computed((): string => {
  if (phase.value === 'running') return '扫描中';
  if (phase.value === 'stopping') return '停止收尾中';
  if (phase.value === 'blocked') return '等待其他任务结束';
  if (outcome.value?.status === 'completed') return '扫描完成';
  if (outcome.value?.status === 'stopped') return '已停止，保留已有结果';
  if (outcome.value?.status === 'failed') return '扫描未完成';
  return '等待开始';
});

async function toggleScan(): Promise<void> {
  if (buttonDisabled.value) return;
  commandBusy.value = true;
  commandError.value = null;
  try {
    if (canBegin.value) {
      await task.tryStart({ workerType: essenceScanWorkerType.value });
    } else if (canStop.value) {
      await task.tryStop();
    }
  } catch (error) {
    commandError.value = error instanceof Error ? error.message : String(error);
  } finally {
    commandBusy.value = false;
  }
}
</script>

<template>
  <UContainer class="py-5">
    <div class="space-y-4">
      <header class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="flex items-center gap-2 text-xl font-semibold">
            <UIcon class="text-primary" name="i-lucide-gem" />基质扫描
          </h1>
          <p class="mt-1 text-sm text-muted">遍历背包中的武器基质，按已保存的规则给出保留建议。</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <UBadge
            v-if="essenceScanWorkerType === 'simulation'"
            color="warning"
            icon="i-lucide-flask-conical"
            label="模拟模式"
            variant="soft"
          />
          <UButton
            :color="
              isActive ? 'error' : essenceScanWorkerType === 'simulation' ? 'warning' : 'primary'
            "
            :disabled="buttonDisabled"
            :icon="isActive ? 'i-lucide-square' : 'i-lucide-play'"
            :label="buttonLabel"
            :loading="commandBusy || phase === 'stopping'"
            @click="toggleScan"
          />
        </div>
      </header>

      <UAlert
        v-if="!desktop"
        color="neutral"
        description="请打开桌面应用来读取设置、运行真实扫描或模拟扫描。"
        icon="i-lucide-monitor-off"
        title="浏览器预览模式"
        variant="subtle"
      />
      <UAlert
        v-else-if="essenceScanWorkerType === 'simulation'"
        color="warning"
        description="固定 54 份样例会使用当前已保存的规则判断，约 20 秒完成，可随时停止。在设置的开发者选项中切换模拟模式。"
        icon="i-lucide-flask-conical"
        title="正在使用模拟基质"
        variant="subtle"
      />
      <UAlert
        v-else
        color="info"
        description="将游戏客户区设为 1280×720，按 N 打开贵重品库并选择「武器基质」。扫描会先回到列表顶部，再依次遍历整个背包。"
        icon="i-lucide-info"
        title="扫描前准备"
        variant="subtle"
      />

      <UAlert
        v-if="catalogError"
        :actions="[{ label: '重新加载目录', onClick: initEssenceCatalog }]"
        color="error"
        :description="catalogError"
        title="属性和武器目录加载失败"
      />
      <UAlert
        v-if="settingsError"
        :actions="[{ label: '重新加载设置', onClick: initOeaSettings }]"
        color="error"
        :description="settingsError"
        title="扫描规则加载失败"
      />
      <UAlert
        v-if="rulesUnsaved && readySettings?.saveError"
        :actions="[{ label: '重试保存规则', onClick: retrySettingsSave }]"
        color="error"
        :description="readySettings.saveError.message"
        title="当前规则尚未保存，保存成功后才能开始扫描"
      />
      <UAlert
        v-if="commandError"
        color="error"
        :description="commandError"
        title="扫描操作失败，请重试"
      />
      <UAlert
        v-if="scanError"
        color="error"
        :description="scanError"
        title="基质扫描未完成，已有结果已保留"
      />

      <EssenceScanRules :disabled="phase !== 'idle'" />

      <section aria-label="扫描统计" class="grid grid-cols-2 gap-2 sm:grid-cols-5">
        <div class="rounded-lg border border-default bg-default p-3">
          <p class="text-xs text-muted">已扫描</p>
          <p class="mt-1 text-2xl font-semibold tabular-nums">{{ summary.total }}</p>
        </div>
        <UButton
          v-for="value in decisions"
          :key="value"
          :aria-pressed="decision === value"
          class="block p-3 text-start"
          :color="decisionPresentation[value].color"
          :variant="decision === value ? 'subtle' : 'soft'"
          @click="decision = decision === value ? 'all' : value"
        >
          <span class="flex items-center gap-1 text-xs"
            ><UIcon :name="decisionPresentation[value].icon" />{{
              decisionPresentation[value].label
            }}</span
          >
          <span class="mt-1 block text-2xl font-semibold tabular-nums">{{ summary[value] }}</span>
        </UButton>
      </section>

      <section aria-label="基质扫描结果" class="space-y-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2 text-sm">
            <span class="font-medium">{{ statusLabel }}</span>
            <UBadge
              v-if="isActive"
              color="primary"
              icon="i-lucide-loader-circle"
              label="实时更新"
              variant="soft"
            />
          </div>
          <div class="flex flex-wrap gap-2">
            <UInput
              v-model="query"
              aria-label="搜索基质结果"
              class="w-60"
              icon="i-lucide-search"
              placeholder="搜索属性、武器或序号"
            />
            <USelect
              v-model="decision"
              aria-label="筛选判断结果"
              class="w-36"
              :items="decisionOptions"
            />
            <UButton
              color="neutral"
              :disabled="isActive || scannedItems.length === 0"
              icon="i-lucide-trash-2"
              label="清空结果"
              variant="outline"
              @click="clearScannedItems"
            />
          </div>
        </div>
        <p v-if="scannedItems.length" class="text-xs text-muted">
          显示 {{ filteredItems.length }} /
          {{ scannedItems.length }} 份基质。点击统计卡可以筛选判断结果。
        </p>
        <div
          v-if="scannedItems.length === 0"
          class="flex min-h-64 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-default text-center"
        >
          <UIcon
            class="size-10 text-dimmed"
            :name="isActive ? 'i-lucide-scan-line' : 'i-lucide-gem'"
          />
          <p class="font-medium">{{ isActive ? '等待第一份基质结果' : '准备好后开始扫描' }}</p>
          <p class="max-w-md px-4 text-sm text-muted">
            {{
              isActive
                ? '正在定位背包并识别基质，结果会逐条显示在这里。'
                : '每份结果都会显示属性等级、匹配武器和判断原因。扫描结束或切换页面后仍可查看。'
            }}
          </p>
        </div>
        <div
          v-else-if="filteredItems.length === 0"
          class="rounded-lg border border-dashed border-default p-10 text-center text-sm text-muted"
        >
          没有符合当前筛选条件的基质。
        </div>
        <UScrollArea
          v-else
          v-slot="{ item }"
          class="h-[60vh] min-h-80 scrollbar-gutter-stable"
          :items="filteredItems"
          :virtualize="{ estimateSize: 170, overscan: 5 }"
        >
          <EssenceResultCard :catalog="catalog" :item="item" />
        </UScrollArea>
      </section>
    </div>
  </UContainer>
</template>
