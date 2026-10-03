<script setup lang="ts">
import { computed, type DeepReadonly } from 'vue';
import { openImagePreview } from '@/composables/image-preview';
import {
  decisionPresentation,
  reasonLabels,
  rarityLabels,
} from '@/features/essenceScan/resultView';
import type { EssenceCatalog, ScannedItem } from '@/features/essenceScan/types';

const { item, catalog } = defineProps<{
  item: DeepReadonly<ScannedItem>;
  catalog: DeepReadonly<EssenceCatalog> | null;
}>();

const decision = computed(() => decisionPresentation[item.evaluation.decision]);
const stats = computed(() =>
  item.essence.stats.map((id, index) => ({
    name: id === null ? '未知属性' : (catalog?.stats.find((stat) => stat.id === id)?.name ?? id),
    level: item.essence.levels[index],
  })),
);
const weapons = computed((): string[] =>
  item.evaluation.matchedWeaponIds.map(
    (id) => catalog?.weapons.find((weapon) => weapon.id === id)?.name ?? id,
  ),
);

function previewImage(): void {
  if (item.image === null) return;
  openImagePreview({
    url: item.image,
    name: `基质 #${item.sequence} 详情`,
    downloadName: `基质-${item.sequence}.png`,
  });
}
</script>

<template>
  <article class="mb-2 flex gap-3 rounded-lg border border-default bg-default p-3 sm:gap-4">
    <UButton
      v-if="item.image"
      :aria-label="`预览基质 ${item.sequence} 的详情截图`"
      class="h-24 w-22 shrink-0 overflow-hidden p-0!"
      color="neutral"
      variant="ghost"
      @click="previewImage"
    >
      <img
        :alt="`基质 ${item.sequence} 详情截图`"
        class="h-full w-full object-cover"
        :src="item.image"
      />
    </UButton>
    <div
      v-else
      class="flex h-24 w-22 shrink-0 flex-col items-center justify-center gap-1 rounded-md bg-muted text-dimmed"
    >
      <UIcon class="size-7" name="i-lucide-gem" />
      <span class="text-xs">无截图</span>
    </div>
    <div class="min-w-0 flex-1 space-y-2">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-sm font-semibold tabular-nums">#{{ item.sequence }}</span>
        <UBadge
          :color="decision.color"
          :icon="decision.icon"
          :label="decision.label"
          variant="subtle"
        />
        <UBadge color="neutral" :label="rarityLabels[item.essence.rarity]" variant="soft" />
        <span class="text-xs text-dimmed"
          >第 {{ item.page }} 页 · 背包第 {{ item.row }} 行 / {{ item.column }} 列</span
        >
      </div>
      <div class="grid gap-1 text-sm sm:grid-cols-3 sm:gap-3">
        <div v-for="(stat, index) in stats" :key="index" class="flex items-center gap-1.5">
          <span :class="stat.name === '未知属性' ? 'text-warning' : 'text-toned'">{{
            stat.name
          }}</span>
          <span
            class="shrink-0 font-semibold tabular-nums"
            :class="stat.level == null ? 'text-warning' : 'text-default'"
          >
            {{ stat.level == null ? '等级未知' : `+${stat.level}` }}
          </span>
        </div>
      </div>
      <p class="text-sm text-toned">{{ reasonLabels[item.evaluation.reason] }}</p>
      <p v-if="weapons.length" class="text-xs text-muted">匹配武器：{{ weapons.join('、') }}</p>
      <div class="flex flex-wrap gap-3 text-xs text-muted">
        <span class="inline-flex items-center gap-1">
          <UIcon
            :name="
              item.essence.locked === null
                ? 'i-lucide-circle-help'
                : item.essence.locked
                  ? 'i-lucide-lock-keyhole'
                  : 'i-lucide-lock-keyhole-open'
            "
          />
          {{
            item.essence.locked === null
              ? '锁定状态未知'
              : item.essence.locked
                ? '已锁定'
                : '未锁定'
          }}
        </span>
        <span>{{
          item.essence.abandoned === null
            ? '弃用状态未知'
            : item.essence.abandoned
              ? '已标记弃用'
              : '未标记弃用'
        }}</span>
      </div>
    </div>
  </article>
</template>
