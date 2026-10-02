<script setup lang="ts">
import { archiveScanWorkerType } from '@/features/archiveScan/workerType';
import {
  developerInstallBusy,
  developerInstallTrace,
  developerInstallUnavailable,
  developerInstallUpdatePackage,
} from '@/features/update/developerUpdate';

import SettingsItem from './SettingsItem.vue';

const { unsupported = false } = defineProps<{ unsupported?: boolean }>();
</script>

<template>
  <div>
    <UAlert
      color="warning"
      icon="i-lucide-triangle-alert"
      title="如果你不知道自己在做什么，请不要使用下面的选项"
      variant="subtle"
    />
  </div>
  <SettingsItem
    description="用固定示例结果调试档案扫描页面，无需游戏窗口。约 20 秒完成，可随时停止。重启应用后关闭。"
    icon="i-lucide-scan-text"
    title="模拟档案扫描"
  >
    <UCheckbox
      color="warning"
      label="使用模拟扫描结果"
      :model-value="archiveScanWorkerType === 'simulation'"
      @update:model-value="archiveScanWorkerType = $event === true ? 'simulation' : 'production'"
    />
  </SettingsItem>
  <SettingsItem
    description="从给定的 .zip 更新包运行一次原地更新流程。支持增量包和全量包。"
    icon="i-lucide-flask-conical"
    title="安装更新包"
  >
    <UBadge v-if="unsupported" color="neutral" label="浏览器中不可用" variant="soft" />
    <div v-else class="flex w-96 flex-col items-end gap-2">
      <UButton
        color="warning"
        :disabled="developerInstallUnavailable"
        icon="i-lucide-package-open"
        label="选择 .zip 文件安装包（开发者）"
        :loading="developerInstallBusy"
        @click="developerInstallUpdatePackage"
      />
      <pre
        class="max-h-52 w-full overflow-auto rounded-md bg-muted p-2 text-xs whitespace-pre-wrap text-toned"
        >{{
          developerInstallTrace.length > 0 ? developerInstallTrace.join('\n') : '相关日志'
        }}</pre>
    </div>
  </SettingsItem>
</template>
