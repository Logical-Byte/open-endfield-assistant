<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { useTheme } from '@/composables/useTheme';
import { useColorMode } from '@vueuse/core';
import { ref, useTemplateRef } from 'vue';

const { t } = useAppI18n();

const colorModeCalculated = useColorMode();
const colorModeRaw = useColorMode({ emitAuto: true });

const open = ref(false);

const activeColorRole = ref<number>(0);
const carousel = useTemplateRef('carousel');

function onTabChange(val: string | number) {
  carousel.value?.emblaApi?.scrollTo(Number(val));
}

function onCarouselSelect(index: number) {
  activeColorRole.value = index;
}

const {
  primaryColors,
  neutralColors,
  radiuses,
  cornerShapePresets,
  supportsCornerShape,
  englishFontOptions,
  chineseFontOptions,
  monospaceFontOptions,
  colorModes,
  primary,
  neutral,
  radius,
  cornerShape,
  cornerShapeCoefficient,
  englishFont,
  chineseFont,
  monospaceFont,
  loadFontCss,
  getPreviewFontFamily,
  resetTheme,
} = useTheme();
</script>

<template>
  <UPopover
    v-model:open="open"
    :ui="{
      content:
        'flex max-h-[calc(100dvh-var(--ui-header-height)-var(--ui-title-height)-1rem)] flex-col gap-4 overflow-y-auto p-4 inline-80 max-inline-[calc(100svw-1rem)]',
    }"
  >
    <UTooltip :text="t('theme.change')">
      <UButton
        :aria-label="t('theme.change')"
        color="neutral"
        icon="i-lucide-palette"
        square
        :ui="{ leadingIcon: 'text-primary' }"
        :variant="open ? 'soft' : 'ghost'"
      />
    </UTooltip>

    <template #content>
      <UFormField>
        <UTabs
          v-model="activeColorRole"
          class="mbe-2"
          :content="false"
          :items="[
            { label: t('theme.primary'), value: 0 },
            { label: t('theme.neutral'), value: 1 },
          ]"
          size="xs"
          variant="link"
          @update:model-value="onTabChange"
        />

        <UCarousel
          ref="carousel"
          v-slot="{ item }"
          auto-height
          :duration="20"
          :items="[0, 1]"
          :ui="{ container: 'transition-[height]' }"
          @select="onCarouselSelect"
        >
          <div v-if="item === 0" class="grid grid-cols-3 gap-1">
            <ThemePickerButton
              v-for="{ id, lightLabel, darkLabel, chipStyle } in primaryColors"
              :key="id"
              :chip-style
              class="capitalize"
              :label="colorModeCalculated === 'dark' ? darkLabel : lightLabel"
              :selected="primary === id"
              @click="primary = id"
            />
          </div>
          <div v-else-if="item === 1" class="grid grid-cols-3 gap-1">
            <ThemePickerButton
              v-for="{ id, lightLabel, darkLabel, chipStyle } in neutralColors"
              :key="id"
              :chip-style
              class="capitalize"
              :label="colorModeCalculated === 'dark' ? darkLabel : lightLabel"
              :selected="neutral === id"
              @click="neutral = id"
            />
          </div>
        </UCarousel>
      </UFormField>

      <UFormField :label="t('theme.radius.title')">
        <div class="grid grid-cols-5 gap-1">
          <ThemePickerButton
            v-for="{ value, label } in radiuses"
            :key="value"
            class="justify-center px-0"
            :label="label"
            :selected="radius === value"
            :style="{
              borderRadius: `${value * cornerShapeCoefficient}rem`,
            }"
            @click="radius = value"
          />
        </div>
      </UFormField>

      <UFormField v-if="supportsCornerShape" :label="t('theme.cornerShape')">
        <div class="grid grid-cols-5 gap-1">
          <ThemePickerButton
            v-for="{ label, value, cssValue, coefficient } in cornerShapePresets"
            :key="value"
            class="justify-center px-0"
            :label="label"
            :selected="cornerShape === value"
            :style="{
              cornerShape: cssValue,
              borderRadius: `${radius * coefficient}rem`,
            }"
            @click="cornerShape = value"
          />
        </div>
      </UFormField>

      <UFormField :label="t('theme.englishFont')">
        <div>
          <USelect
            v-model="englishFont"
            class="inline-full"
            :content="{ bodyLock: false }"
            icon="i-lucide-type"
            :items="englishFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadFontCss(englishFontOptions);
              }
            "
          >
            <template #item-label="{ item }">
              <span :style="{ fontFamily: getPreviewFontFamily('english', item) }">
                {{ item.label }}
              </span>
            </template>
          </USelect>
        </div>
      </UFormField>

      <UFormField :label="t('theme.chineseFont')">
        <div>
          <USelect
            v-model="chineseFont"
            class="inline-full"
            :content="{ bodyLock: false }"
            icon="i-lucide-quote"
            :items="chineseFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadFontCss(chineseFontOptions);
              }
            "
          >
            <template #item-label="{ item }">
              <span :style="{ fontFamily: getPreviewFontFamily('chinese', item) }">
                {{ item.label }}
              </span>
            </template>
          </USelect>
        </div>
      </UFormField>

      <UFormField :label="t('theme.monospaceFont')">
        <div>
          <USelect
            v-model="monospaceFont"
            class="font-mono inline-full"
            :content="{ bodyLock: false }"
            icon="i-lucide-code"
            :items="monospaceFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadFontCss(monospaceFontOptions);
              }
            "
          >
            <template #item-label="{ item }">
              <span :style="{ fontFamily: getPreviewFontFamily('monospace', item) }">
                {{ item.label }}
              </span>
            </template>
          </USelect>
        </div>
      </UFormField>

      <UFormField :label="t('theme.colorMode')">
        <div class="grid grid-cols-3 gap-1">
          <ThemePickerButton
            v-for="{ label, value, icon } in colorModes"
            :key="value"
            :icon="icon"
            :label="label"
            :selected="colorModeRaw === value"
            @click="colorModeRaw = value"
          />
        </div>
      </UFormField>

      <UFormField>
        <div class="flex justify-end">
          <UTooltip :text="t('theme.reset')">
            <UButton
              :aria-label="t('theme.reset')"
              class="ring-default hover:bg-elevated/50"
              color="neutral"
              icon="i-lucide-rotate-ccw"
              size="sm"
              variant="outline"
              @click="resetTheme"
            />
          </UTooltip>
        </div>
      </UFormField>
    </template>
  </UPopover>
</template>
