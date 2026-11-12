<script setup lang="ts">
defineProps<{ icon: string; spinning: boolean; progress: number | null }>();
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
  <UIcon v-else class="size-5 shrink-0" :class="spinning && 'animate-spin'" :name="icon" />
</template>
