<script setup lang="ts">
import { ref } from 'vue';

/**
 * 更新提醒（样板，纯静态）。
 *
 * 实际逻辑中，仅在检测到新版本时由父组件（`AppHeader`）通过 `v-if` 渲染；
 * 新版本号、更新日志与下载设置均由后端更新检查结果填充。
 */
const updateOpen = ref(false);
const downloadOpen = ref(false);
const settingsOpen = ref(false);

/** 当前版本与新版本（样板数据）。 */
const currentVersion = ref('0.1.0');
const newVersion = ref('0.2.0');

/** 更新日志（样板数据）。 */
const releaseNote = ref([
  '新增：档案库扫描结果支持按获取方式筛选',
  '优化：大幅提升 OCR 识别速度，降低误识别率',
  '修复：部分分辨率下截图窗口误判的问题',
]);

/** 下载相关设置（样板数据）。 */
const downloadSource = ref('mirrorchyan');
const downloadProxyMode = ref('system');

const downloadSourceItems = [
  { label: 'Mirror酱', value: 'mirrorchyan' },
  { label: 'GitHub', value: 'github' },
];

const downloadProxyModeItems = [
  { label: '不使用代理', value: 'none' },
  { label: '系统代理', value: 'system' },
  { label: '自定义代理', value: 'custom' },
];

function startDownload() {
  updateOpen.value = false;
  downloadOpen.value = true;
}
</script>

<template>
  <UPopover v-model:open="updateOpen">
    <UTooltip text="发现新版本">
      <span class="relative inline-flex">
        <UButton
          aria-label="发现新版本"
          color="neutral"
          icon="i-lucide-cloud-download"
          square
          :variant="updateOpen ? 'soft' : 'ghost'"
        />
        <span class="absolute top-0.5 right-0.5 size-2 rounded-full bg-primary" />
      </span>
    </UTooltip>

    <template #content>
      <div class="flex w-sm flex-col gap-3 p-4">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <UIcon class="text-xl text-primary" name="i-lucide-circle-arrow-up" />
            <p class="font-semibold">发现新版本</p>
          </div>
          <div class="flex items-center gap-1.5">
            <UBadge color="neutral" variant="soft">v{{ currentVersion }}</UBadge>
            <UIcon class="text-toned" name="i-lucide-arrow-right" />
            <UBadge color="primary" variant="soft">v{{ newVersion }}</UBadge>
          </div>
        </div>

        <div class="rounded-md bg-muted p-3">
          <p class="mb-2 text-xs font-medium text-toned">更新日志</p>
          <ul class="flex list-disc flex-col gap-1 ps-4 text-sm">
            <li v-for="item in releaseNote" :key="item">{{ item }}</li>
          </ul>
        </div>

        <div class="flex w-full gap-2">
          <UButton
            class="flex-1 justify-center"
            icon="i-lucide-download"
            label="立即更新"
            @click="startDownload"
          />
          <UPopover v-model:open="settingsOpen">
            <UButton aria-label="下载设置" icon="i-lucide-settings-2" variant="subtle" />
            <template #content>
              <div class="w-64 space-y-4 p-4">
                <UFormField label="下载源">
                  <USelect v-model="downloadSource" class="w-full" :items="downloadSourceItems" />
                </UFormField>

                <UFormField v-if="downloadSource === 'mirrorchyan'" label="Mirror酱 CDK">
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
                    class="w-full"
                    color="neutral"
                    placeholder="未填写时使用 GitHub 下载"
                    type="password"
                  />
                </UFormField>

                <UFormField label="下载代理">
                  <USelect
                    v-model="downloadProxyMode"
                    class="w-full"
                    :items="downloadProxyModeItems"
                  />
                </UFormField>

                <UFormField v-if="downloadProxyMode === 'custom'" label="自定义代理">
                  <UInput class="w-full" placeholder="http://127.0.0.1:7890" />
                </UFormField>
              </div>
            </template>
          </UPopover>
        </div>
      </div>
    </template>
  </UPopover>

  <UpdateModal v-model="downloadOpen" />
</template>
