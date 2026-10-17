<script setup lang="ts">
import { computed, type Ref } from 'vue';
import { useColorMode } from '@vueuse/core';
import { isTauri } from '@tauri-apps/api/core';
import { useTheme } from '@/composables/useTheme';
import { useAppI18n } from '@/shared/i18n';
import { uiScale } from '@/features/appearance/uiScale';
import type { FontOption } from '@/features/appearance/fonts';
import type { SettingsItemState, SliderSettingsItem } from './settingsItem';
import SettingsCard from './SettingsCard.vue';
import SettingsItem from './SettingsItem.vue';
import ThemeColorPicker from './ThemeColorPicker.vue';

const { t, n } = useAppI18n();
const colorMode = useColorMode({ emitAuto: true });
const {
  primaryColors,
  neutralColors,
  primary,
  neutral,
  colorModes,
  radiuses,
  radius,
  cornerShapePresets,
  cornerShape,
  cornerShapeCoefficient,
  supportsCornerShape,
  englishFont,
  chineseFont,
  monospaceFont,
  englishFontOptions,
  chineseFontOptions,
  monospaceFontOptions,
  loadFontCss,
  getPreviewFontFamily,
  resetThemeColors,
  resetThemeFonts,
  resetThemeShape,
} = useTheme();

interface FontField {
  type: 'chinese' | 'english' | 'monospace';
  label: string;
  icon: string;
  value: Ref<string>;
  options: FontOption[];
  sample: string;
}

// 样例用于比较对应字形，保持固定文本，不随应用语言翻译。
const fontFields = computed<FontField[]>((): FontField[] => [
  {
    type: 'chinese',
    label: t('theme.chineseFont'),
    icon: 'i-lucide-quote',
    value: chineseFont,
    options: chineseFontOptions.value,
    sample: '受困者的录音，探索塔卫二的故事。',
  },
  {
    type: 'english',
    label: t('theme.englishFont'),
    icon: 'i-lucide-type',
    value: englishFont,
    options: englishFontOptions.value,
    sample: 'The quick brown fox jumps over the lazy dog.',
  },
  {
    type: 'monospace',
    label: t('theme.monospaceFont'),
    icon: 'i-lucide-code',
    value: monospaceFont,
    options: monospaceFontOptions.value,
    sample: 'Archive scan 00:24  0123456789',
  },
]);

function selectedFontFamily(field: FontField): string | undefined {
  const option = field.options.find(
    (font: FontOption): boolean => font.value === field.value.value,
  );
  return option ? getPreviewFontFamily(field.type, option) : undefined;
}

// WebView 缩放独立于后端设置。加载或保存后端设置失败时，桌面端仍可调整。
const scaleItemState = computed<SettingsItemState>((): SettingsItemState =>
  isTauri()
    ? { status: 'ready' }
    : { status: 'unavailable', label: t('settings.unsupported.label'), variant: 'subtle' },
);
const scaleItem: SliderSettingsItem = {
  kind: 'slider',
  state: scaleItemState,
  read: (): number => uiScale.value,
  write: (value: number): void => {
    uiScale.value = value;
  },
  min: 0.5,
  max: 2,
  step: 0.05,
  marks: [0.5, 1, 1.5, 2],
  tooltip: true,
  formatValue: (value: number): string => n(value, 'percent'),
};

function resetColors(): void {
  resetThemeColors();
  colorMode.value = 'auto';
}
function resetShape(): void {
  resetThemeShape();
  uiScale.value = 1;
}
</script>

<template>
  <div class="space-y-4">
    <SettingsCard icon="i-lucide-palette" :title="t('settings.interface.appearance.section')">
      <template #actions>
        <UTooltip :text="t('settings.interface.appearance.reset')">
          <UButton
            :aria-label="t('settings.interface.appearance.reset')"
            color="neutral"
            icon="i-lucide-rotate-ccw"
            :label="t('settings.interface.restoreDefaults')"
            size="sm"
            variant="ghost"
            @click="resetColors"
          />
        </UTooltip>
      </template>
      <SettingsItem
        :description="t('settings.interface.appearance.modeDescription')"
        icon="i-lucide-sun-moon"
        :title="t('theme.colorMode')"
      >
        <div class="flex shrink-0 flex-wrap gap-2">
          <UButton
            v-for="mode in colorModes"
            :key="mode.value"
            :aria-pressed="colorMode === mode.value"
            :icon="mode.icon"
            :label="mode.label"
            :variant="colorMode === mode.value ? 'solid' : 'outline'"
            @click="colorMode = mode.value"
          />
        </div>
      </SettingsItem>
      <SettingsItem
        :description="t('settings.interface.appearance.primaryDescription')"
        icon="i-lucide-palette"
        :title="t('theme.primary')"
      >
        <ThemeColorPicker v-model="primary" :items="primaryColors" />
      </SettingsItem>
      <SettingsItem
        :description="t('settings.interface.appearance.neutralDescription')"
        icon="i-lucide-paint-bucket"
        :title="t('theme.neutral')"
      >
        <ThemeColorPicker v-model="neutral" :items="neutralColors" />
      </SettingsItem>
    </SettingsCard>

    <SettingsCard icon="i-lucide-type" :title="t('settings.interface.fonts.section')">
      <template #actions>
        <UTooltip :text="t('settings.interface.fonts.reset')">
          <UButton
            :aria-label="t('settings.interface.fonts.reset')"
            color="neutral"
            icon="i-lucide-rotate-ccw"
            :label="t('settings.interface.restoreDefaults')"
            size="sm"
            variant="ghost"
            @click="resetThemeFonts"
          />
        </UTooltip>
      </template>
      <SettingsItem
        v-for="font in fontFields"
        :key="font.type"
        :icon="font.icon"
        :title="font.label"
      >
        <template #description
          ><p class="mt-1 text-sm text-dimmed" :style="{ fontFamily: selectedFontFamily(font) }">
            {{ font.sample }}
          </p></template
        >
        <USelect
          v-model="font.value.value"
          :aria-label="font.label"
          class="w-full shrink-0 sm:w-56"
          :content="{ bodyLock: false }"
          :items="font.options"
          @update:open="
            (open: boolean): void => {
              if (open) loadFontCss(font.options);
            }
          "
        >
          <template #item-label="{ item }"
            ><span :style="{ fontFamily: getPreviewFontFamily(font.type, item) }">{{
              item.label
            }}</span></template
          >
        </USelect>
      </SettingsItem>
    </SettingsCard>

    <SettingsCard icon="i-lucide-scan" :title="t('settings.interface.shape.section')">
      <template #actions>
        <UTooltip :text="t('settings.interface.shape.reset')">
          <UButton
            :aria-label="t('settings.interface.shape.reset')"
            color="neutral"
            icon="i-lucide-rotate-ccw"
            :label="t('settings.interface.restoreDefaults')"
            size="sm"
            variant="ghost"
            @click="resetShape"
          />
        </UTooltip>
      </template>
      <SettingsItem
        :description="t('settings.interface.shape.radiusDescription')"
        icon="i-lucide-radius"
        :title="t('theme.radius.title')"
      >
        <div class="flex shrink-0 flex-wrap gap-2">
          <UButton
            v-for="item in radiuses"
            :key="item.value"
            :aria-pressed="radius === item.value"
            :label="item.label"
            :style="{ borderRadius: `${item.value * cornerShapeCoefficient}rem` }"
            :variant="radius === item.value ? 'solid' : 'outline'"
            @click="radius = item.value"
          />
        </div>
      </SettingsItem>
      <SettingsItem
        v-if="supportsCornerShape"
        icon="i-lucide-square-round-corner"
        :title="t('theme.cornerShape')"
      >
        <div class="flex shrink-0 flex-wrap gap-2">
          <UButton
            v-for="item in cornerShapePresets"
            :key="item.value"
            :aria-pressed="cornerShape === item.value"
            :label="item.label"
            :style="{ cornerShape: item.cssValue, borderRadius: `${radius * item.coefficient}rem` }"
            :variant="cornerShape === item.value ? 'solid' : 'outline'"
            @click="cornerShape = item.value"
          />
        </div>
      </SettingsItem>
      <SettingsItem
        :description="t('settings.interface.scale.description')"
        icon="i-lucide-zoom-in"
        :item="scaleItem"
        :title="t('settings.interface.scale.title')"
      />
    </SettingsCard>
  </div>
</template>
