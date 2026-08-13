<script setup lang="ts">
import { renderMarkdown } from '@/utils/markdown';
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

/** 更新日志（样板数据，Markdown 源码，用于测试超长内容滚动）。 */
const releaseNote = ref(`# v0.2.0 更新日志

> 本次更新内容较多，请仔细阅读。

## 新增功能

- **档案库扫描**：扫描结果支持按获取方式筛选
- **自动更新**：应用内一键检查并下载新版本，支持 Mirror 酱镜像源与 GitHub 官方源
- **下载代理**：支持系统代理、自定义代理与不使用代理三种模式
- **扫描音效**：扫描开始、完成、失败时播放提示音，音量可调
- **主题系统**：支持主色、次色、中性色、圆角与字体自定义

### 档案库扫描

- 新增「按获取方式筛选」下拉框
- 商店兑换类档案支持跳转 OEM 页面
- 扫描结果卡片支持缩放预览

### 自动更新

- 检测到新版本时在标题栏显示提醒气泡
- 更新日志支持 Markdown 渲染，内容过长可滚动查看

## 优化

- 大幅提升 OCR 识别速度，降低误识别率约 30%
- 优化截图窗口定位逻辑，部分高 DPI 场景不再误判
- 减少应用启动时的内存占用

## 修复

- 修复部分分辨率下截图窗口误判的问题
- 修复深浅色主题切换后部分颜色未即时生效的问题
- 修复扫描中断后进度条未复位的问题

## 使用说明

### 命令行构建

\`\`\`bash
pnpm install
pnpm dev
pnpm build
\`\`\`

### 打包发布

使用 \`pnpm package\` 生成绿色便携版压缩包，产物位于 \`release/\` 目录。

## 已知问题

1. 部分 Windows 版本首次启动需等待 WebView2 安装完成
2. 极少数分辨率下截图窗口仍可能误判，欢迎反馈
3. Mirror 酱镜像源需要付费，GitHub 官方源免费但速度可能较慢

## 更新路线图

| 版本 | 主要内容 | 状态 |
| ---- | -------- | ---- |
| v0.1.0 | 基础扫描 | 已发布 |
| v0.2.0 | 自动更新 | 本次 |
| v0.3.0 | 多显示器 | 计划中 |

- [x] 档案库扫描
- [x] 自动更新
- [ ] 多显示器支持
- [ ] 云同步

---

感谢使用 OEA，反馈问题请前往 [GitHub Issues](https://github.com/Logical-Byte/open-endfield-assistant/issues)。
`);

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
  <UPopover
    v-model:open="updateOpen"
    :ui="{
      content: 'flex max-h-[calc(100svh-var(--ui-header-height)-2rem)] w-md flex-col gap-3 p-4',
    }"
  >
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

      <div class="flex min-h-0 flex-col gap-2 rounded-md bg-muted p-3 pr-1 ring ring-default">
        <p class="text-xs font-medium text-toned">更新日志</p>
        <!-- eslint-disable vue/no-v-html 渲染结果经 DOMPurify 消毒 -->
        <div
          class="markdown-body max-h-128 min-h-0 scrollbar-gutter-stable overflow-y-auto pr-1"
          v-html="renderMarkdown(releaseNote)"
        />
        <!-- eslint-enable vue/no-v-html -->
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
                    ><ULink class="text-primary hover:text-primary/75" to="https://mirrorchyan.com/"
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
    </template>
  </UPopover>

  <UpdateModal v-model="downloadOpen" />
</template>
