<script setup lang="ts">
import { computed, ref } from 'vue';
import { catalog, catalogStatus } from '@/features/essenceScan/catalog';
import type { EssenceScanSettings, StatKind } from '@/features/essenceScan/types';
import {
  editSettings,
  initOeaSettings,
  retrySettingsSave,
  settingsState,
} from '@/features/settings/settings';

const { disabled = false } = defineProps<{ disabled?: boolean }>();
const open = ref(false);
const newRule = ref<[string | undefined, string | undefined, string | undefined]>([
  undefined,
  undefined,
  undefined,
]);
const ready = computed(() => (settingsState.value.status === 'ready' ? settingsState.value : null));
const draft = computed(() => ready.value?.draft.essenceScan ?? null);
const canEdit = computed(
  (): boolean => !disabled && draft.value !== null && catalogStatus.value === 'ready',
);
const unsaved = computed(
  (): boolean =>
    ready.value !== null && ready.value.draft.essenceScan !== ready.value.effective.essenceScan,
);
const initializeError = computed((): string | null =>
  settingsState.value.status === 'unavailable' &&
  settingsState.value.reason.type === 'initialize-error'
    ? settingsState.value.reason.error.message
    : null,
);
const saveLabel = computed((): string =>
  ready.value?.saveError ? '保存失败' : unsaved.value ? '正在保存' : '已保存',
);
const slots: { index: 0 | 1 | 2; kind: StatKind; label: string; max: number }[] = [
  { index: 0, kind: 'attribute', label: '主属性', max: 6 },
  { index: 1, kind: 'secondary', label: '次属性', max: 6 },
  { index: 2, kind: 'skill', label: '技能', max: 3 },
];
const statOptions = computed(() =>
  slots.map((slot) =>
    (catalog.value?.stats ?? [])
      .filter((stat) => stat.kind === slot.kind)
      .map((stat) => ({ label: stat.name, value: stat.id })),
  ),
);
const weaponOptions = computed(() =>
  (catalog.value?.weapons ?? []).map((weapon) => ({
    label: `${weapon.name} · ${weapon.rarity}星`,
    value: weapon.id,
  })),
);
const nonFiveStarOptions: { label: string; value: EssenceScanSettings['nonFiveStar'] }[] = [
  { label: '按相同规则判断', value: 'process' },
  { label: '直接跳过', value: 'skip' },
];
const excludedWeapons = computed<string[]>({
  get: () => [...(draft.value?.excludedWeaponIds ?? [])],
  set: (value: string[]) => editRules({ excludedWeaponIds: value }),
});
const duplicateRule = computed(
  (): boolean =>
    draft.value?.customKeeps.some((rule) =>
      rule.every((id, index) => id === newRule.value[index]),
    ) ?? false,
);
const canAddRule = computed(
  (): boolean =>
    canEdit.value && newRule.value.every((id) => id !== undefined) && !duplicateRule.value,
);

function editRules(patch: Partial<EssenceScanSettings>): void {
  if (!canEdit.value || draft.value === null) return;
  editSettings({ essenceScan: { ...draft.value, ...patch } });
}

function setThreshold(index: 0 | 1 | 2, value: number | null | undefined): void {
  const current = draft.value?.highLevel;
  if (current == null || value == null || !Number.isFinite(value)) return;
  const thresholds: [number, number, number] = [...current];
  thresholds[index] = Math.max(1, Math.min(index === 2 ? 3 : 6, Math.round(value)));
  editRules({ highLevel: thresholds });
}

function addCustomRule(): void {
  const [attribute, secondary, skill] = newRule.value;
  if (
    !canAddRule.value ||
    attribute === undefined ||
    secondary === undefined ||
    skill === undefined ||
    draft.value === null
  )
    return;
  editRules({ customKeeps: [...draft.value.customKeeps, [attribute, secondary, skill]] });
  newRule.value = [undefined, undefined, undefined];
}

function removeCustomRule(index: number): void {
  if (draft.value === null) return;
  editRules({ customKeeps: draft.value.customKeeps.filter((_, i) => i !== index) });
}

function ruleLabel(rule: readonly string[]): string {
  return rule
    .map((id) => catalog.value?.stats.find((stat) => stat.id === id)?.name ?? id)
    .join(' + ');
}
</script>

<template>
  <UCollapsible v-model:open="open" class="rounded-lg border border-default bg-default">
    <UButton class="w-full justify-between px-4 py-3" color="neutral" variant="ghost">
      <span class="flex items-center gap-2 font-medium"
        ><UIcon name="i-lucide-sliders-horizontal" />保留规则</span
      >
      <span class="flex items-center gap-2 text-xs text-muted">
        <UBadge
          v-if="draft?.autoMark"
          color="warning"
          label="自动标记已开启"
          size="sm"
          variant="soft"
        />
        <span v-if="draft" class="hidden sm:inline"
          >排除 {{ draft.excludedWeaponIds.length }} 把武器 ·
          {{ draft.customKeeps.length }} 个自定义组合</span
        >
        <UBadge
          v-if="ready"
          :color="ready.saveError ? 'error' : unsaved ? 'warning' : 'neutral'"
          :label="saveLabel"
          size="sm"
          variant="soft"
        />
        <UIcon :name="open ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'" />
      </span>
    </UButton>
    <template #content>
      <div class="space-y-5 border-t border-default p-4">
        <UAlert
          v-if="initializeError"
          :actions="[{ label: '重试加载设置', onClick: initOeaSettings }]"
          color="error"
          :description="initializeError"
          title="保留规则加载失败"
        />
        <UAlert
          v-else-if="!draft"
          color="neutral"
          description="请在桌面应用中加载设置后编辑规则。"
          :title="settingsState.status === 'initializing' ? '正在加载保留规则' : '保留规则暂不可用'"
        />
        <template v-else>
          <UAlert
            v-if="ready?.saveError"
            :actions="[{ label: '重试保存', onClick: retrySettingsSave }]"
            color="error"
            :description="ready.saveError.message"
            title="修改尚未保存，扫描仍使用最近一次成功保存的规则"
          />
          <p class="text-xs text-muted">
            修改会自动保存，成功后用于下一次扫描。识别不完整的基质会留待确认。{{
              disabled ? '自动化任务运行期间暂时不能修改规则。' : ''
            }}
          </p>
          <div class="space-y-2 rounded-md bg-muted p-3">
            <USwitch
              :disabled="!canEdit"
              label="自动锁定或标记弃用"
              :model-value="draft.autoMark"
              @update:model-value="editRules({ autoMark: $event })"
            />
            <p class="text-xs text-muted">
              默认关闭。开启后，保留的基质自动锁定，建议丢弃的未锁定基质标记弃用，每次操作后重新识别确认。
              跳过和待确认项目保持原状。已锁定的基质不会自动解锁，扫描不执行分解。
            </p>
          </div>
          <div class="grid gap-5 md:grid-cols-3">
            <UFormField label="非五星基质">
              <USelect
                class="w-full"
                :disabled="!canEdit"
                :items="nonFiveStarOptions"
                :model-value="draft.nonFiveStar"
                @update:model-value="editRules({ nonFiveStar: $event })"
              />
            </UFormField>
            <div class="space-y-1">
              <USwitch
                :disabled="!canEdit"
                label="保护已锁定基质"
                :model-value="draft.protectLocked"
                @update:model-value="editRules({ protectLocked: $event })"
              />
              <p class="text-xs text-muted">已锁定时直接给出保留建议。</p>
            </div>
            <div class="space-y-1">
              <USwitch
                :disabled="!canEdit"
                label="跳过已弃用基质"
                :model-value="draft.skipAbandoned"
                @update:model-value="editRules({ skipAbandoned: $event })"
              />
              <p class="text-xs text-muted">已标记弃用时跳过规则判断。</p>
            </div>
          </div>
          <div class="space-y-3 border-t border-default pt-4">
            <USwitch
              :disabled="!canEdit"
              label="保留高等级基质"
              :model-value="draft.highLevel !== null"
              @update:model-value="editRules({ highLevel: $event ? [3, 3, 3] : null })"
            />
            <p class="text-xs text-muted">
              任一属性达到对应阈值就保留，包括仅匹配已排除武器的基质。
            </p>
            <div v-if="draft.highLevel" class="grid max-w-xl grid-cols-3 gap-3">
              <UFormField v-for="slot in slots" :key="slot.kind" :label="`${slot.label}等级`">
                <UInputNumber
                  class="w-full"
                  :disabled="!canEdit"
                  :max="slot.max"
                  :min="1"
                  :model-value="draft.highLevel[slot.index]"
                  @update:model-value="setThreshold(slot.index, $event)"
                />
              </UFormField>
            </div>
          </div>
          <UFormField
            description="仅匹配这些武器时建议丢弃。高等级和自定义保留规则仍然有效。"
            label="排除武器"
          >
            <USelectMenu
              v-model="excludedWeapons"
              class="w-full"
              :disabled="!canEdit"
              :items="weaponOptions"
              multiple
              placeholder="选择不需要保留基质的武器"
              :search-input="{ placeholder: '搜索武器名称' }"
              value-key="value"
            />
          </UFormField>
          <div class="space-y-3 border-t border-default pt-4">
            <div>
              <p class="text-sm font-medium">自定义保留组合</p>
              <p class="text-xs text-muted">
                三个属性全部匹配时保留，按属性类型判断，与游戏内的显示顺序无关。
              </p>
            </div>
            <div class="grid items-end gap-2 sm:grid-cols-[1fr_1fr_1fr_auto]">
              <UFormField v-for="slot in slots" :key="slot.kind" :label="slot.label">
                <USelectMenu
                  v-model="newRule[slot.index]"
                  class="w-full"
                  :disabled="!canEdit"
                  :items="statOptions[slot.index]"
                  :placeholder="`选择${slot.label}`"
                  :search-input="{ placeholder: '搜索属性名称' }"
                  value-key="value"
                />
              </UFormField>
              <UButton
                :disabled="!canAddRule"
                icon="i-lucide-plus"
                :label="duplicateRule ? '组合已存在' : '添加组合'"
                @click="addCustomRule"
              />
            </div>
            <ul v-if="draft.customKeeps.length" class="space-y-2">
              <li
                v-for="(rule, index) in draft.customKeeps"
                :key="rule.join('/')"
                class="flex items-center justify-between gap-2 rounded-md bg-muted px-3 py-2 text-sm"
              >
                <span>{{ ruleLabel(rule) }}</span>
                <UButton
                  :aria-label="`删除组合 ${ruleLabel(rule)}`"
                  color="error"
                  :disabled="!canEdit"
                  icon="i-lucide-x"
                  size="xs"
                  variant="ghost"
                  @click="removeCustomRule(index)"
                />
              </li>
            </ul>
            <p v-else class="text-xs text-dimmed">尚未添加自定义组合。</p>
          </div>
        </template>
      </div>
    </template>
  </UCollapsible>
</template>
