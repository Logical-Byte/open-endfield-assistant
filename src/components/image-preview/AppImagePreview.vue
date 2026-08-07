<script setup lang="ts">
import { useImagePreview } from '@/composables/image-preview/useImagePreview';
import { app } from '@/main';
import { openImagePreviewKey } from '@/utils/provideInject';
import { useTemplateRef } from 'vue';

const overlayRef = useTemplateRef('overlayRef');
const imgRef = useTemplateRef('imgRef');

const {
  preview,
  scale,
  backgroundColor,
  naturalWidth,
  naturalHeight,
  imgStyle,
  onImageLoad,
  open,
  close,
  download,
  zoomIn,
  zoomOut,
  rotateClockwise,
  resetView,
  onWheel,
  onMousedown,
  onMousemove,
  onMouseup,
  onKeydown,
} = useImagePreview(overlayRef, imgRef);

app.provide(openImagePreviewKey, open);
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-opacity duration-200"
      enter-from-class="opacity-0"
      leave-active-class="transition-opacity duration-150"
      leave-to-class="opacity-0"
    >
      <div
        v-if="preview"
        ref="overlayRef"
        class="fixed inset-0 flex flex-col outline-none"
        :style="{ backgroundColor }"
        tabindex="0"
        @keydown="onKeydown"
        @mouseleave="onMouseup"
        @mousemove="onMousemove"
        @mouseup="onMouseup"
      >
        <!-- 顶部工具栏 -->
        <div
          class="flex shrink-0 items-center justify-end gap-4 bg-default px-4 py-2 sm:justify-between"
        >
          <div class="hidden min-w-0 flex-1 flex-col sm:flex">
            <p class="truncate text-sm font-medium text-highlighted">{{ preview.name }}</p>
            <ULink
              class="truncate text-xs text-muted"
              rel="noopener noreferrer"
              target="_blank"
              :to="preview.url"
            >
              {{ preview.url.slice(0, 256) }}
            </ULink>
          </div>
          <p
            v-if="naturalWidth && naturalHeight"
            class="hidden shrink-0 text-sm whitespace-nowrap text-muted sm:block"
          >
            {{ naturalWidth }} × {{ naturalHeight }}
          </p>
          <div class="flex shrink-0 items-center gap-1">
            <UTooltip :kbds="['-']" text="缩小">
              <UButton color="neutral" icon="i-lucide-minus" variant="ghost" @click="zoomOut" />
            </UTooltip>
            <UTooltip :kbds="['0']" text="重置视图">
              <UButton
                class="min-w-16 justify-center text-sm"
                color="neutral"
                variant="ghost"
                @click="resetView()"
              >
                {{ Math.round(scale * 100) }}%
              </UButton>
            </UTooltip>
            <UTooltip :kbds="['=']" text="放大">
              <UButton color="neutral" icon="i-lucide-plus" variant="ghost" @click="zoomIn" />
            </UTooltip>
            <UTooltip :kbds="['R']" text="顺时针旋转 90°">
              <UButton
                color="neutral"
                icon="i-lucide-rotate-cw"
                variant="ghost"
                @click="rotateClockwise"
              />
            </UTooltip>
            <div class="mx-1 h-5 w-px bg-accented" />
            <UPopover mode="click" :ui="{ content: 'p-4' }">
              <UTooltip text="更改背景颜色">
                <UButton color="neutral" icon="i-lucide-palette" variant="ghost" />
              </UTooltip>
              <template #content>
                <UColorPicker v-model="backgroundColor" />
              </template>
            </UPopover>
            <UTooltip :kbds="['O']" text="在新标签页中打开图像">
              <UButton
                color="neutral"
                icon="i-lucide-external-link"
                rel="noopener noreferrer"
                target="_blank"
                :to="preview.url"
                variant="ghost"
              />
            </UTooltip>
            <UTooltip :kbds="['meta', 'S']" text="下载">
              <UButton color="neutral" icon="i-lucide-download" variant="ghost" @click="download" />
            </UTooltip>
            <UPopover mode="click" :ui="{ content: 'w-80 p-4' }">
              <UTooltip text="帮助">
                <UButton
                  color="neutral"
                  icon="i-lucide-circle-question-mark"
                  size="lg"
                  variant="ghost"
                />
              </UTooltip>
              <template #content>
                <ImagePreviewHelpMenu />
              </template>
            </UPopover>
            <UTooltip :kbds="['escape']" text="关闭">
              <UButton color="neutral" icon="i-lucide-x" variant="ghost" @click="close" />
            </UTooltip>
          </div>
        </div>

        <!-- 图片展示区 -->
        <div
          class="relative flex flex-1 items-center justify-center overflow-hidden select-none"
          @click.self="close"
          @wheel.prevent="onWheel"
        >
          <img
            ref="imgRef"
            :alt="preview.name"
            class="max-w-none"
            draggable="false"
            referrerpolicy="no-referrer"
            :src="preview.url"
            :style="imgStyle"
            @load="onImageLoad"
            @mousedown="onMousedown"
          />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
