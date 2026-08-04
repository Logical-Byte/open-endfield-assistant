<script setup lang="ts">
import LogViewer from '@/components/LogViewer.vue';
import { openLogDir } from '@/lib/tauri';
import { ref } from 'vue';

const logViewer = ref<InstanceType<typeof LogViewer> | null>(null);

function openLogDirectory(): void {
  void openLogDir();
}

function clearLog(): void {
  logViewer.value?.clear();
}
</script>

<template>
  <UContainer>
    <UPage>
      <UPageBody>
        <div class="flex flex-wrap gap-2">
          <UButton
            icon="i-lucide-folder-open"
            label="打开日志文件目录"
            size="lg"
            @click="openLogDirectory"
          />
          <UButton
            color="error"
            icon="i-lucide-trash-2"
            label="清空日志"
            size="lg"
            variant="outline"
            @click="clearLog"
          />
        </div>

        <LogViewer ref="logViewer" />
      </UPageBody>
    </UPage>
  </UContainer>
</template>
