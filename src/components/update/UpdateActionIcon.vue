<script setup lang="ts">
defineProps<{ icon: string; spinning: boolean; progress: number | null; ready: boolean }>();
</script>
<template>
  <svg
    v-if="progress !== null"
    aria-hidden="true"
    class="size-5 shrink-0 -rotate-90"
    viewBox="0 0 24 24"
  >
    <circle
      cx="12"
      cy="12"
      fill="none"
      opacity="0.2"
      r="9"
      stroke="currentColor"
      stroke-width="2"
    />
    <circle
      cx="12"
      cy="12"
      fill="none"
      pathLength="100"
      r="9"
      stroke="currentColor"
      :stroke-dasharray="`${progress} 100`"
      stroke-linecap="round"
      stroke-width="2"
    />
  </svg>
  <span v-else aria-hidden="true" class="relative grid size-5 shrink-0 place-items-center">
    <UIcon class="size-5" :class="spinning && 'animate-spin'" :name="icon" />
    <span
      v-if="ready"
      class="absolute -right-0.5 -bottom-0.5 grid size-3 place-items-center rounded-full bg-current"
    >
      <UIcon class="size-2 text-inverted" name="i-lucide-check" />
    </span>
  </span>
</template>
