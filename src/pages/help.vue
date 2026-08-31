<script setup lang="ts">
import type { AccordionItem } from '@nuxt/ui';
import { onBeforeUnmount, onMounted, ref } from 'vue';

/** 常见问题（含链接的条目通过自定义 slot 渲染）。 */
const faqItems: AccordionItem[] = [
  {
    label: '手机能用吗？',
    content: '不能。OEA 仅支持 Windows 10 / 11（x86_64）。',
  },
  {
    label: '识别结果不准确怎么办？',
    content: '可以使用输入框进行人工纠错。建议将识别错误告知我们，以便改进识别算法。',
  },
  {
    label: 'OEA 收费吗？',
    slot: 'faq-fee',
  },
  {
    label: 'OEA 和 Mirror酱的关系是什么？',
    slot: 'faq-mirror',
  },
];

/** 文档目录：`id` 同时用作滚动锚点。 */
const sections = [
  { id: 'getting-started', icon: 'i-lucide-rocket', title: '新手提示' },
  { id: 'usage', icon: 'i-lucide-keyboard', title: '操作说明' },
  { id: 'known-issues', icon: 'i-lucide-triangle-alert', title: '已知问题' },
  { id: 'faq', icon: 'i-lucide-circle-help', title: '常见问题' },
  { id: 'feedback', icon: 'i-lucide-message-circle', title: '反馈交流' },
  { id: 'credits', icon: 'i-lucide-heart', title: '致谢' },
  { id: 'disclaimer', icon: 'i-lucide-file-text', title: '说明' },
];

/** 当前高亮的文档分类 id。 */
const activeSectionId = ref<string>('getting-started');

/** 点击目录触发程序化滚动期间，暂停滚动监听，避免平滑滚动途中高亮抖动。 */
let isProgrammaticScroll = false;

/** 更新当前高亮分类：取顶部越过阈值、最靠下的分类。 */
function updateActiveSection(): void {
  if (isProgrammaticScroll) {
    return;
  }
  const offset = 120;
  let current = sections[0].id;
  for (const section of sections) {
    const el = document.getElementById(section.id);
    if (el !== null && el.getBoundingClientRect().top <= offset) {
      current = section.id;
    }
  }
  activeSectionId.value = current;
}

/** 点击目录跳转到对应分类并立即高亮。 */
function scrollToSection(id: string): void {
  activeSectionId.value = id;
  isProgrammaticScroll = true;
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  window.setTimeout(() => {
    isProgrammaticScroll = false;
  }, 700);
}

onMounted(() => {
  // `UMain` 渲染为 <main>，是实际滚动容器。
  document
    .querySelector('main')
    ?.addEventListener('scroll', updateActiveSection, { passive: true });
});

onBeforeUnmount(() => {
  document.querySelector('main')?.removeEventListener('scroll', updateActiveSection);
});
</script>

<template>
  <UContainer class="min-h-[calc(100lvh-var(--ui-header-height)-var(--ui-title-height))]">
    <UPage>
      <template #left>
        <UPageAside
          :ui="{
            root: 'lg:sticky lg:top-0 lg:max-h-[calc(100lvh-var(--ui-header-height)-var(--ui-title-height))] lg:overflow-y-auto',
          }"
        >
          <nav class="flex flex-col gap-1">
            <UButton
              v-for="section in sections"
              :key="section.id"
              class="w-full justify-start"
              :color="activeSectionId === section.id ? 'primary' : 'neutral'"
              :icon="section.icon"
              :label="section.title"
              :variant="activeSectionId === section.id ? 'soft' : 'ghost'"
              @click="scrollToSection(section.id)"
            />
          </nav>
        </UPageAside>
      </template>

      <UPageBody class="space-y-8">
        <!-- 头部：官网 / GitHub / QQ 交流群 -->
        <div class="flex flex-wrap gap-2">
          <UButton
            icon="i-lucide-globe"
            label="OEA 官网"
            rel="noopener noreferrer"
            target="_blank"
            to="https://ef.yituliu.cn/resources/oea"
          />
          <UButton
            color="neutral"
            icon="i-simple-icons:github"
            label="GitHub 仓库"
            rel="noopener noreferrer"
            target="_blank"
            to="https://github.com/Logical-Byte/open-endfield-assistant"
          />
          <UButton
            icon="i-simple-icons:qq"
            label="反馈交流群：954628501"
            rel="noopener noreferrer"
            target="_blank"
            to="https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms"
          />
        </div>

        <!-- 新手提示 -->
        <UCard id="getting-started" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-rocket" />
              <span class="font-semibold text-highlighted">新手提示</span>
            </div>
          </template>
          <ol class="flex flex-col gap-4 text-lg leading-relaxed font-medium">
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">1.</span>
              <span>
                打开终末地，调成 <strong class="text-primary">1280 × 720</strong>、<strong
                  class="text-primary"
                  >简体中文</strong
                >
              </span>
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">2.</span>
              <span><strong class="text-primary">关闭 HDR</strong>，关闭性能监控软件</span>
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">3.</span>
              <span>终末地打开<strong class="text-primary">档案库界面</strong></span>
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">4.</span>
              <span>点击左上角<strong class="text-primary">开始扫描</strong></span>
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">5.</span>
              <span>扫完点击右上角<strong class="text-primary">导出到地图集</strong></span>
            </li>
          </ol>
        </UCard>

        <!-- 操作说明 -->
        <UCard id="usage" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-keyboard" />
              <span class="font-semibold text-highlighted">操作说明</span>
            </div>
          </template>
          <div class="flex flex-col gap-6">
            <div>
              <p class="mbe-3 font-medium text-highlighted">使用前准备</p>
              <ul class="flex list-disc flex-col gap-3 ps-6 text-muted marker:text-muted">
                <li>
                  理论上支持任意 <strong class="text-primary">16:9</strong> 的分辨率。我们最建议使用
                  <strong class="text-primary">1280 × 720</strong>、<strong class="text-primary"
                    >窗口模式</strong
                  >，这个分辨率可以兼顾准确性和性能。
                </li>
                <li>
                  理论上目前支持从任意档案库界面、协议终端界面和大世界界面开始扫描，为了稳定性，建议始终从<strong
                    class="text-primary"
                    >档案库主界面</strong
                  >开始扫描。
                </li>
                <li>请将终末地的语言调成<strong class="text-primary">简体中文</strong>。</li>
                <li>
                  请<strong class="text-primary">关闭 HDR</strong>，关闭任何会遮挡终末地窗口的软件。
                </li>
              </ul>
            </div>
            <div>
              <p class="mbe-3 font-medium text-highlighted">快捷键</p>
              <ul class="flex list-disc flex-col gap-3 ps-6 text-muted marker:text-muted">
                <li>按 <UKbd>'</UKbd>（引号键）开始扫描档案库；扫描过程中再次按下可停止</li>
                <li>按 <UKbd>Alt</UKbd> + <UKbd>Delete</UKbd> 退出程序</li>
              </ul>
            </div>
          </div>
        </UCard>

        <!-- 已知问题 -->
        <UCard id="known-issues" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-triangle-alert" />
              <span class="font-semibold text-highlighted">已知问题</span>
            </div>
          </template>
          <ol class="list-disc space-y-3 text-muted">
            存在 2 个不同的档案，名称都为「挂在竹子上的字条」。OEA
            目前无法区分二者，目前只要识别到其一就认为 2 个档案都已收集。
          </ol>
        </UCard>

        <!-- 常见问题 -->
        <UCard id="faq" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-circle-help" />
              <span class="font-semibold text-highlighted">常见问题</span>
            </div>
          </template>
          <UAccordion :items="faqItems">
            <template #faq-fee>
              <p class="pb-3.5 text-sm text-muted">
                OEA 开源且免费，不会以任何形式收取费用。
                <br />
                您可以前往
                <ULink
                  class="text-primary hover:text-primary/75"
                  rel="noopener noreferrer"
                  target="_blank"
                  to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                  >GitHub Release</ULink
                >
                免费下载和使用 OEA。
                <br />
                如果您是通过付费方式获取的 OEA，您可能已经被不法商家欺骗，请立即告知我们。
              </p>
            </template>
            <template #faq-mirror>
              <p class="pb-3.5 text-sm text-muted">
                <ULink
                  class="text-primary hover:text-primary/75"
                  rel="noopener noreferrer"
                  target="_blank"
                  to="https://mirrorchyan.com/"
                  >Mirror酱</ULink
                >
                是独立的第三方应用分发平台，提供加速下载服务，需要付费使用。OEA
                本身不收取任何费用，也提供免费的下载渠道，您可以前往
                <ULink
                  class="text-primary hover:text-primary/75"
                  rel="noopener noreferrer"
                  target="_blank"
                  to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                  >GitHub Release</ULink
                >
                免费下载和使用。
              </p>
            </template>
          </UAccordion>
        </UCard>

        <!-- 反馈交流 -->
        <UCard id="feedback" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-message-circle" />
              <span class="font-semibold text-highlighted">反馈交流</span>
            </div>
          </template>
          <div class="flex flex-col gap-4">
            <div class="flex flex-wrap gap-2">
              <UButton
                icon="i-simple-icons:qq"
                label="反馈交流群：954628501"
                rel="noopener noreferrer"
                target="_blank"
                to="https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms"
              />
              <UButton
                color="neutral"
                icon="i-simple-icons:github"
                label="提交 GitHub Issue"
                rel="noopener noreferrer"
                target="_blank"
                to="https://github.com/Logical-Byte/open-endfield-assistant/issues"
              />
            </div>
            <p class="text-sm text-muted">
              遇到问题或建议，欢迎反馈并附上应用目录下
              <code>logs/</code> 中的日志文件，便于定位问题。
            </p>
          </div>
        </UCard>

        <!-- 致谢 -->
        <UCard id="credits" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-heart" />
              <span class="font-semibold text-highlighted">致谢</span>
            </div>
          </template>
          <div class="flex flex-col items-start gap-1">
            <UButton
              class="px-0"
              color="primary"
              label="终末地一图流"
              rel="noopener noreferrer"
              target="_blank"
              to="https://ef.yituliu.cn/"
              trailing-icon="i-lucide-external-link"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="终末地地图集"
              rel="noopener noreferrer"
              target="_blank"
              to="https://oem.re/"
              trailing-icon="i-lucide-external-link"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="逻辑元LogicalByte"
              rel="noopener noreferrer"
              target="_blank"
              to="https://space.bilibili.com/688411531"
              trailing-icon="i-lucide-external-link"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="Mirror酱"
              rel="noopener noreferrer"
              target="_blank"
              to="https://mirrorchyan.com/"
              trailing-icon="i-lucide-external-link"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="RapidAI/RapidOCR (GitHub)"
              rel="noopener noreferrer"
              target="_blank"
              to="https://github.com/RapidAI/RapidOCR"
              trailing-icon="i-simple-icons:github"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="RapidAI/RapidOCR 模型 (ModelScope)"
              rel="noopener noreferrer"
              target="_blank"
              to="https://www.modelscope.cn/models/RapidAI/RapidOCR"
              trailing-icon="i-lucide-external-link"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="MaaXYZ/MaaFramework"
              rel="noopener noreferrer"
              target="_blank"
              to="https://github.com/MaaXYZ/MaaFramework"
              trailing-icon="i-simple-icons:github"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="MistEO/MXU"
              rel="noopener noreferrer"
              target="_blank"
              to="https://github.com/MistEO/MXU"
              trailing-icon="i-simple-icons:github"
              variant="link"
            />
            <UButton
              class="px-0"
              color="primary"
              label="MaaEnd/MaaEnd"
              rel="noopener noreferrer"
              target="_blank"
              to="https://github.com/MaaEnd/MaaEnd"
              trailing-icon="i-simple-icons:github"
              variant="link"
            />
          </div>
        </UCard>

        <!-- 说明 -->
        <UCard id="disclaimer" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-file-text" />
              <span class="font-semibold text-highlighted">说明</span>
            </div>
          </template>
          <ol class="list-disc space-y-3 ps-6 text-muted marker:text-muted">
            <li>
              自动更新功能有删除硬盘上的文件的操作，请确保重要数据已备份再使用自动更新功能，避免误删重要文件。
            </li>
            <li>机器识别，可能存在错误。若发现错误，欢迎反馈。</li>
            <li>
              本工具按 “原样”、“包含全部错误” 和 “视可用性情况”
              提供，作者不对可用性、准确性或使用效果做出任何承诺或保证。
            </li>
            <li>
              使用者必须确保使用本工具符合相关法律法规与服务条款，禁止用于任何违法或侵权行为。
            </li>
            <li>使用者需承担因使用本工具产生的任何风险、损失或责任。</li>
            <li>使用本工具即意味着您同意以上全部内容。</li>
          </ol>
        </UCard>
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
