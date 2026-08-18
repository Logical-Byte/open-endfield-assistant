<script setup lang="ts">
import { UpdateInstallStatus } from '@/types/update';
import {
  closeInstallModal,
  installError,
  installStage,
  installStageLabel,
  installStatus,
  justUpdatedInfo,
  retryInstall,
  showInstallModal,
} from '@/utils/app/update';
import { renderMarkdown } from '@/utils/markdown';
import { computed } from 'vue';

/** 是否正在安装（弹窗不可关闭）。 */
const isInstalling = computed(() => installStatus.value === UpdateInstallStatus.Installing);
/** 是否安装失败。 */
const isFailed = computed(() => installStatus.value === UpdateInstallStatus.Failed);
/** 是否安装完成（即将重启 / 重启失败）。 */
const isComplete = computed(() => installStatus.value === UpdateInstallStatus.Completed);
/** 是否「更新完成」展示模式（重启后）。 */
const isJustUpdatedMode = computed(() => justUpdatedInfo.value !== null);
/** 是否允许通过 X / 遮罩关闭：安装中不允许；失败、重启失败、更新完成可关闭。 */
const canClose = computed(
  () =>
    !isInstalling.value &&
    (isFailed.value ||
      (isComplete.value && installError.value !== null) ||
      isJustUpdatedMode.value),
);
</script>

<template>
  <UModal
    v-if="showInstallModal"
    v-model:open="showInstallModal"
    :close="canClose"
    :dismissible="!isInstalling"
  >
    <template #body>
      <div class="flex flex-col items-center gap-4 py-6 text-center">
        <!-- 重启后的「更新完成」展示 -->
        <template v-if="isJustUpdatedMode">
          <UIcon class="size-12 text-success" name="i-lucide-circle-check" />
          <div class="space-y-1">
            <p class="font-semibold">更新完成</p>
            <p class="text-sm text-toned">
              v{{ justUpdatedInfo?.previousVersion }} → {{ justUpdatedInfo?.newVersion }}
            </p>
          </div>
          <!-- eslint-disable vue/no-v-html 渲染结果经 DOMPurify 消毒 -->
          <div
            class="markdown-body max-h-80 w-full overflow-y-auto rounded-md bg-muted p-3 text-left text-sm"
            v-html="renderMarkdown(justUpdatedInfo?.releaseNote ?? '暂无更新日志')"
          />
          <!-- eslint-enable vue/no-v-html -->
          <UButton label="知道了" @click="closeInstallModal" />
        </template>

        <!-- 安装中（不可关闭） -->
        <template v-else-if="isInstalling">
          <UIcon class="size-12 animate-spin text-primary" name="i-lucide-loader-circle" />
          <div class="space-y-1">
            <p class="font-semibold">正在安装更新</p>
            <p class="text-sm text-toned">
              {{ installStage ? installStageLabel(installStage) : '准备中…' }}
            </p>
          </div>
          <UProgress class="w-full" size="sm" :value="null" />
        </template>

        <!-- 安装完成：正常情况即将自动重启；重启失败则提示手动重启 -->
        <template v-else-if="isComplete">
          <UIcon class="size-12 text-success" name="i-lucide-circle-check" />
          <div class="space-y-1">
            <p class="font-semibold">安装完成</p>
            <p v-if="installError" class="text-sm text-error">{{ installError }}</p>
            <p v-else class="text-sm text-toned">正在重启应用…</p>
          </div>
          <UButton v-if="installError" label="关闭" @click="closeInstallModal" />
        </template>

        <!-- 安装失败 -->
        <template v-else-if="isFailed">
          <UIcon class="size-12 text-error" name="i-lucide-circle-alert" />
          <div class="space-y-1">
            <p class="font-semibold">安装失败</p>
            <p class="text-sm whitespace-pre-wrap text-error">{{ installError }}</p>
          </div>
          <div class="flex gap-2">
            <UButton color="neutral" label="关闭" variant="soft" @click="closeInstallModal" />
            <UButton
              color="primary"
              icon="i-lucide-refresh-cw"
              label="重试"
              @click="retryInstall"
            />
          </div>
        </template>
      </div>
    </template>
  </UModal>
</template>
