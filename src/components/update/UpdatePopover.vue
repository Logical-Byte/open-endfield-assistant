<script setup lang="ts">
import { oeaVersion } from '@/main';
import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import { UpdateCheckStatus, UpdateDownloadStatus } from '@/types/update';
import { appStatus } from '@/utils/app/appStatus';
import { mirrorchyanCdk, oeaConfig, proxyModeItems, updateSourceItems } from '@/utils/app/config';
import {
  cancelDownload,
  checkUpdate,
  downloadProgress,
  downloadStatus,
  startDownload,
  startInstall,
  updateCheckResult,
} from '@/utils/app/update';
import { renderMarkdown } from '@/utils/markdown';
import { updatePopoverOpen } from '@/utils/uiState';
import { computed, ref } from 'vue';

const settingsOpen = ref(false);

/** 新版本号（服务端 version_name 带 v 前缀，此处去掉以统一显示 v 前缀）。 */
const newVersion = computed<string | null>(() => {
  if (updateCheckResult.value.status === UpdateCheckStatus.HasUpdate) {
    const version = updateCheckResult.value.result.data?.version_name;
    return version ? version.replace(/^v/, '') : null;
  } else {
    return null;
  }
});

const releaseNote = computed<string | null>(() => {
  if (updateCheckResult.value.status === UpdateCheckStatus.HasUpdate) {
    return updateCheckResult.value.result.data?.release_note ?? null;
  } else {
    return null;
  }
});

const maybeStatusChipColor = computed<string | null>(() => {
  switch (updateCheckResult.value.status) {
    case UpdateCheckStatus.HasUpdate:
      return 'bg-success';
    case UpdateCheckStatus.Error:
      return 'bg-error';
    default:
      return null;
  }
});

/** 按钮 tooltip 与 aria-label 的状态文案。 */
const statusText = computed<string>(() => {
  switch (updateCheckResult.value.status) {
    case UpdateCheckStatus.Idle:
      return '检查更新';
    case UpdateCheckStatus.Checking:
      return '正在检查更新';
    case UpdateCheckStatus.HasUpdate:
      return '发现新版本';
    case UpdateCheckStatus.NoUpdate:
      return '当前已是最新版本';
    case UpdateCheckStatus.Error:
      return '检查更新失败';
    default:
      return '检查更新';
  }
});

/** 下载进度文案：已下载 / 总大小（总大小未知时只显示已下载）。 */
const progressText = computed<string>(() => {
  const { downloadedSize, totalSize } = downloadProgress.value;
  if (totalSize > 0) {
    return `${formatBytes(downloadedSize)} / ${formatBytes(totalSize)}`;
  }
  return formatBytes(downloadedSize);
});

/** 剩余时间估算（速度 > 0 且总大小已知时）。 */
const etaText = computed<string | null>(() => {
  const { downloadedSize, totalSize, speed } = downloadProgress.value;
  if (totalSize <= 0 || speed <= 0 || downloadedSize >= totalSize) {
    return null;
  }
  const seconds = Math.ceil((totalSize - downloadedSize) / speed);
  if (seconds < 60) {
    return ` · 约 ${seconds} 秒`;
  }
  if (seconds < 3600) {
    return ` · 约 ${Math.ceil(seconds / 60)} 分钟`;
  }
  return ` · 约 ${(seconds / 3600).toFixed(1)} 小时`;
});

/** 字节数格式化为人类可读单位。 */
function formatBytes(bytes: number): string {
  if (bytes <= 0) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  return `${value.toFixed(value >= 100 || index === 0 ? 0 : 1)} ${units[index]}`;
}

/** 速度格式化为人类可读单位。 */
function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}
</script>

<template>
  <UPopover
    v-if="
      [UpdateCheckStatus.HasUpdate, UpdateCheckStatus.Error, UpdateCheckStatus.Checking].includes(
        updateCheckResult.status,
      )
    "
    v-model:open="updatePopoverOpen"
    :ui="{
      content:
        'flex max-h-[calc(100dvh-var(--ui-header-height)-var(--ui-title-height)-1rem)] w-md flex-col gap-3 p-4',
    }"
  >
    <UTooltip :text="statusText">
      <div class="relative">
        <UButton
          :aria-label="statusText"
          color="neutral"
          icon="i-lucide-cloud-download"
          :loading="updateCheckResult.status === UpdateCheckStatus.Checking"
          :variant="updatePopoverOpen ? 'soft' : 'ghost'"
        />
        <span
          v-if="maybeStatusChipColor"
          class="absolute top-0 right-0 size-2 rounded-full"
          :class="maybeStatusChipColor"
        />
      </div>
    </UTooltip>

    <template #content>
      <template v-if="updateCheckResult.status === UpdateCheckStatus.Checking">
        <div class="flex items-center justify-center gap-2 py-2">
          <UIcon class="size-5 animate-spin text-primary" name="i-lucide-loader-circle" />
          <p class="text-sm font-medium text-toned">正在检查更新…</p>
        </div>
      </template>

      <template v-else-if="updateCheckResult.status === UpdateCheckStatus.Error">
        <div class="flex items-center gap-2">
          <UIcon class="size-5 text-error" name="i-lucide-circle-alert" />
          <p class="font-semibold">检查更新失败</p>
        </div>
        <p class="text-sm whitespace-pre-wrap text-toned">
          {{ updateCheckResult.error.message }}
        </p>
        <UButton block icon="i-lucide-rotate-cw" label="重试" @click="checkUpdate" />
      </template>

      <template v-else-if="updateCheckResult.status === UpdateCheckStatus.HasUpdate">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <UIcon class="text-xl text-primary" name="i-lucide-circle-arrow-up" />
            <p class="font-semibold">发现新版本</p>
          </div>
          <div class="flex items-center gap-1.5">
            <UBadge color="neutral" variant="subtle">v{{ oeaVersion }}</UBadge>
            <UIcon class="text-toned" name="i-lucide-arrow-right" />
            <UBadge color="primary" variant="subtle">v{{ newVersion ?? '未知' }}</UBadge>
          </div>
        </div>

        <div class="flex min-h-0 flex-col gap-2 rounded-md bg-muted p-3 pr-1 ring ring-default">
          <p class="text-xs font-medium text-toned">更新日志</p>
          <!-- eslint-disable vue/no-v-html 渲染结果经 DOMPurify 消毒 -->
          <div
            class="markdown-body max-h-128 min-h-0 scrollbar-gutter-stable overflow-y-auto pr-1"
            v-html="renderMarkdown(releaseNote ?? '暂无更新日志')"
          />
          <!-- eslint-enable vue/no-v-html -->
        </div>

        <!-- 下载状态区 -->
        <div
          v-if="downloadStatus === UpdateDownloadStatus.Downloading"
          class="space-y-2 rounded-md bg-muted p-3"
        >
          <div class="flex items-center justify-between text-xs text-toned">
            <span class="flex items-center gap-1.5">
              <UIcon class="size-3.5 animate-spin" name="i-lucide-loader-circle" />
              正在下载
            </span>
            <span class="tabular-nums">{{ progressText }}</span>
          </div>
          <UProgress size="sm" :value="downloadProgress.progress" />
          <div class="flex items-center justify-between text-xs text-dimmed">
            <span class="tabular-nums"
              >{{ downloadProgress.progress.toFixed(1) }}% · {{ formatSpeed(downloadProgress.speed)
              }}{{ etaText }}</span
            >
            <UButton
              color="neutral"
              label="取消"
              size="xs"
              variant="ghost"
              @click="cancelDownload"
            />
          </div>
        </div>

        <div
          v-else-if="downloadStatus === UpdateDownloadStatus.Cancelling"
          class="flex items-center gap-2 rounded-md bg-muted p-3 text-sm text-toned"
        >
          <UIcon class="size-4 animate-spin text-primary" name="i-lucide-loader-circle" />
          正在取消…
        </div>

        <div v-else-if="downloadStatus === UpdateDownloadStatus.Completed" class="space-y-2">
          <div class="rounded-md bg-success/10 p-3 text-sm text-success">下载完成</div>
          <UButton
            block
            color="primary"
            :disabled="appStatus.running"
            icon="i-lucide-package-check"
            label="立即安装"
            @click="startInstall"
          />
          <p v-if="appStatus.running" class="text-xs text-dimmed">
            扫描任务运行中，扫描结束后将自动安装
          </p>
        </div>

        <div
          v-else-if="downloadStatus === UpdateDownloadStatus.Failed"
          class="flex items-center justify-between gap-2 rounded-md bg-error/10 p-3"
        >
          <p class="text-sm text-error">下载失败</p>
          <UButton color="error" label="重试" size="xs" variant="soft" @click="startDownload" />
        </div>

        <!-- 仅下载前（Idle）显示「立即更新」与下载设置；下载中/已下载/失败均不显示 -->
        <div v-if="downloadStatus === UpdateDownloadStatus.Idle" class="flex w-full gap-2">
          <UButton block icon="i-lucide-download" label="立即更新" @click="startDownload" />
          <UPopover v-model:open="settingsOpen">
            <UButton aria-label="下载设置" icon="i-lucide-settings-2" variant="subtle" />
            <template #content>
              <div class="w-64 space-y-4 p-4">
                <UFormField label="下载源">
                  <USelect
                    v-model="oeaConfig.updateSource"
                    class="w-full"
                    :items="updateSourceItems"
                  />
                </UFormField>

                <UFormField
                  v-if="oeaConfig.updateSource === UpdateSource.Mirrorchyan"
                  label="Mirror酱 CDK"
                >
                  <template #help>
                    <span class="text-xs leading-none text-dimmed"
                      ><ULink
                        class="text-primary hover:text-primary/75"
                        to="https://mirrorchyan.com/"
                        >Mirror酱</ULink
                      >
                      是独立的第三方加速下载服务，需要付费使用。
                      <br />
                      <ULink
                        class="text-primary hover:text-primary/75"
                        rel="noopener noreferrer"
                        target="_blank"
                        to="https://ef.yituliu.cn/resources/oea"
                        >OEA</ULink
                      >
                      本身不收取任何费用，也提供免费的下载渠道。您可以前往
                      <ULink
                        class="text-primary hover:text-primary/75"
                        rel="noopener noreferrer"
                        target="_blank"
                        to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                        >GitHub Release</ULink
                      >
                      免费下载和使用。</span
                    ></template
                  >
                  <template #hint
                    ><ULink
                      class="flex items-center gap-1 text-xs text-primary hover:text-primary/75"
                      rel="noopener noreferrer"
                      target="_blank"
                      to="https://mirrorchyan.com/?source=oea"
                      >获取 CDK<UIcon name="i-lucide-external-link" /></ULink
                  ></template>
                  <UInput
                    v-model="mirrorchyanCdk"
                    class="w-full"
                    placeholder="未填写时使用 GitHub 下载"
                    type="password"
                  />
                </UFormField>

                <UFormField label="下载代理">
                  <USelect
                    v-model="oeaConfig.updateProxyMode"
                    class="w-full"
                    :items="proxyModeItems"
                  />
                </UFormField>

                <UFormField
                  v-if="oeaConfig.updateProxyMode === UpdateProxyMode.Custom"
                  label="自定义代理"
                >
                  <UInput
                    v-model="oeaConfig.updateProxyUrl"
                    class="w-full"
                    placeholder="http://127.0.0.1:7890"
                  />
                </UFormField>
              </div>
            </template>
          </UPopover>
        </div>
      </template>
    </template>
  </UPopover>
</template>
