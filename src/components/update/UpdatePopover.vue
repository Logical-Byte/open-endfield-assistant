<script setup lang="ts">
import { UpdateProxyMode, UpdateSource } from '@/types/oeaConfig';
import { UpdateCheckStatus } from '@/types/update';
import { appVersion } from '@/utils/app/appVersion';
import { mirrorchyanCdk, oeaConfig, proxyModeItems, updateSourceItems } from '@/utils/app/config';
import { checkUpdate, updateCheckResult } from '@/utils/app/update';
import { renderMarkdown } from '@/utils/markdown';
import { downloadingModalOpen, updatePopoverOpen } from '@/utils/uiState';
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

function startDownload() {
  updatePopoverOpen.value = false;
  downloadingModalOpen.value = true;
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
        <p class="text-sm text-toned">
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
            <UBadge color="neutral" variant="subtle">v{{ appVersion ?? '未知' }}</UBadge>
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

        <div class="flex w-full gap-2">
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
                  <UInput class="w-full" placeholder="http://127.0.0.1:7890" />
                </UFormField>
              </div>
            </template>
          </UPopover>
        </div>
      </template>
    </template>
  </UPopover>
</template>
