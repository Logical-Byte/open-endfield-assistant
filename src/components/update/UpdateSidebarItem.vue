<script setup lang="ts">
import SidebarItem from '@/components/layout/SidebarItem.vue';
import { useUpdateAction } from '@/features/update/useUpdateAction';
import UpdateActionIcon from './UpdateActionIcon.vue';
defineProps<{ collapsed: boolean }>();
const { icon, spinning, progress, label, tooltip, disabledReason, activate } = useUpdateAction();
</script>
<template>
  <UTooltip :content="{ side: 'right' }" :disabled="collapsed" :text="tooltip">
    <SidebarItem
      :aria-disabled="!!disabledReason"
      :collapsed="collapsed"
      :icon="icon"
      :label="label"
      :tooltip="tooltip"
      @click="activate"
    >
      <template #icon
        ><UpdateActionIcon :icon="icon" :progress="progress?.progress ?? null" :spinning="spinning"
      /></template>
    </SidebarItem>
  </UTooltip>
</template>
