<template>
  <div class="group/image-preview relative cursor-zoom-in overflow-hidden">
    <div
      class="h-full w-full transition-all group-hover/image-preview:scale-105 group-hover/image-preview:blur-[2px] group-hover/image-preview:brightness-50"
    >
      <slot />
    </div>
    <div
      class="absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover/image-preview:opacity-100"
    >
      <UIcon class="dark size-1/5 min-h-4 min-w-4 text-default" name="i-lucide-zoom-in" />
    </div>
  </div>
</template>
