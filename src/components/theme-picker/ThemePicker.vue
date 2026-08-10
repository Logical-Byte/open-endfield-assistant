<script setup lang="ts">
import { useTheme } from '@/composables/useTheme';
import { useColorMode } from '@vueuse/core';
import { ref, useTemplateRef } from 'vue';

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
  secondaryColors,
  neutralColors,
  radiuses,
  cornerShapePresets,
  supportsCornerShape,
  englishFontOptions,
  chineseFontOptions,
  monospaceFontOptions,
  colorModes,
  primary,
  secondary,
  neutral,
  radius,
  cornerShape,
  cornerShapeCoefficient,
  englishFont,
  chineseFont,
  monospaceFont,
  loadEnglishFontCss,
  loadChineseFontCss,
  loadMonospaceFontCss,
  getPreviewFontFamily,
  resetTheme,
} = useTheme();
</script>

<template>
  <UPopover
    v-model:open="open"
    :ui="{
      content:
        'flex flex-col gap-4 overflow-y-auto p-4 inline-80 max-block-[calc(100svh-6rem)] max-inline-[calc(100svw-1rem)]',
    }"
  >
    <UTooltip text="更改主题">
      <UButton
        aria-label="更改主题"
        color="neutral"
        icon="i-lucide-palette"
        square
        :ui="{ leadingIcon: 'text-primary' }"
        :variant="open ? 'soft' : 'ghost'"
      />
    </UTooltip>

    <template #content>
      <UFormField label="主题色">
        <UTabs
          v-model="activeColorRole"
          class="mbe-2"
          :content="false"
          :items="[
            { label: '第一主题色', value: 0 },
            { label: '第二主题色', value: 1 },
            { label: '中性色', value: 2 },
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
          :items="[0, 1, 2]"
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
              v-for="{ id, lightLabel, darkLabel, chipStyle } in secondaryColors"
              :key="id"
              :chip-style
              class="capitalize"
              :label="colorModeCalculated === 'dark' ? darkLabel : lightLabel"
              :selected="secondary === id"
              @click="secondary = id"
            />
          </div>
          <div v-else-if="item === 2" class="grid grid-cols-3 gap-1">
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

      <UFormField label="圆角大小">
        <div class="grid grid-cols-5 gap-1">
          <ThemePickerButton
            v-for="r in radiuses"
            :key="r"
            class="justify-center px-0"
            :label="String(r)"
            :selected="radius === r"
            :style="{
              borderRadius: `${r * cornerShapeCoefficient}rem`,
            }"
            @click="radius = r"
          />
        </div>
      </UFormField>

      <UFormField v-if="supportsCornerShape" label="圆角形状">
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

      <UFormField label="英文字体">
        <div>
          <USelect
            v-model="englishFont"
            class="inline-full"
            color="neutral"
            :content="{ bodyLock: false }"
            icon="i-lucide-type"
            :items="englishFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadEnglishFontCss();
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

      <UFormField label="中文字体">
        <div>
          <USelect
            v-model="chineseFont"
            class="inline-full"
            color="neutral"
            :content="{ bodyLock: false }"
            icon="i-lucide-quote"
            :items="chineseFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadChineseFontCss();
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

      <UFormField label="等宽字体">
        <div>
          <USelect
            v-model="monospaceFont"
            class="font-mono inline-full"
            color="neutral"
            :content="{ bodyLock: false }"
            icon="i-lucide-code"
            :items="monospaceFontOptions"
            size="sm"
            :ui="{
              trailingIcon: 'transition-transform duration-200 group-data-[state=open]:rotate-180',
            }"
            @update:open="
              (open) => {
                if (open) loadMonospaceFontCss();
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

      <UFormField label="颜色模式">
        <div class="grid grid-cols-3 gap-1">
          <ThemePickerButton
            v-for="{ label, value, icon } in colorModes"
            :key="label"
            :icon="icon"
            :label="label"
            :selected="colorModeRaw === value"
            @click="colorModeRaw = value"
          />
        </div>
      </UFormField>

      <UFormField>
        <div class="flex justify-end">
          <UTooltip text="重置主题">
            <UButton
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
