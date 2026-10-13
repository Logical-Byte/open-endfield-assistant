<script setup lang="ts">
import { computed } from 'vue';
import SettingsControl from './SettingsControl.vue';
import {
  settingsSkeletonClasses,
  type SettingsControl as Control,
  type SettingsItemState,
  type SettingsSkeleton,
} from './settingsItem';

const {
  icon = undefined,
  title = undefined,
  description = undefined,
  state = { status: 'ready' },
  control = undefined,
  skeleton = undefined,
} = defineProps<{
  icon?: string;
  title?: string;
  description?: string;
  state?: SettingsItemState;
  control?: Control;
  skeleton?: SettingsSkeleton;
}>();

const skeletonClass = computed(() => settingsSkeletonClasses[skeleton ?? control?.kind ?? 'input']);
</script>

<template>
  <div class="flex flex-col items-stretch justify-between gap-4 sm:flex-row sm:items-center">
    <div class="flex min-w-0 flex-1 items-center gap-3">
      <slot name="left">
        <slot name="icon">
          <UIcon v-if="icon !== undefined" class="text-2xl text-primary" :name="icon" />
        </slot>
        <div class="min-w-0 flex-1">
          <slot name="title">
            <p v-if="title !== undefined" class="font-medium">{{ title }}</p>
          </slot>
          <slot name="description">
            <p v-if="description !== undefined" class="text-sm text-dimmed">{{ description }}</p>
          </slot>
        </div>
      </slot>
    </div>
    <USkeleton v-if="state.status === 'loading'" :class="skeletonClass" />
    <UBadge
      v-else-if="state.status === 'unavailable'"
      color="neutral"
      :label="state.label"
      :variant="state.variant ?? 'soft'"
    />
    <slot v-else :disabled="state.status === 'disabled'">
      <SettingsControl
        v-if="control"
        :aria-label="title"
        :control="control"
        :disabled="state.status === 'disabled'"
      />
    </slot>
  </div>
</template>
