<script setup lang="ts">
import {
  developerInstallBusy,
  developerInstallTrace,
  developerInstallUnavailable,
  developerInstallUpdatePackage,
} from '@/utils/app/developerUpdate';
</script>

<template>
  <SettingsCard id="developer" class="scroll-mt-8" icon="i-lucide-code-2" title="开发者选项">
    <div>
      <UAlert
        color="warning"
        icon="i-lucide-triangle-alert"
        title="如果你不知道自己在做什么，请不要使用下面的选项"
        variant="subtle"
      />
    </div>
    <SettingsItem
      description="从给定的 .zip 更新包运行一次原地更新流程。支持增量包和全量包。"
      icon="i-lucide-flask-conical"
      title="安装更新包"
    >
      <div class="flex w-96 flex-col items-end gap-2">
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
  </SettingsCard>
</template>
