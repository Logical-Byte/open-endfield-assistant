<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import WorkspaceSection from '@/components/layout/WorkspaceSection.vue';
import { TranslationComponent } from 'i18next-vue';
import type { AccordionItem } from '@nuxt/ui';
import { computed, onBeforeUnmount, onMounted, useTemplateRef, watch } from 'vue';
import {
  activeHelpSectionId,
  helpSectionRequest,
  helpSections,
  type HelpSectionId,
} from '@/app/sectionNavigation';

const { t } = useAppI18n();
const helpPage = useTemplateRef('helpPage');
let scrollContainer: HTMLElement | null = null;

/** 常见问题（含链接的条目通过自定义 slot 渲染）。 */
const faqItems = computed<AccordionItem[]>(() => [
  {
    label: t('help.faq.mobile.question'),
    slot: 'faq-mobile',
  },
  {
    label: t('help.faq.accuracy.question'),
    slot: 'faq-accuracy',
  },
  {
    label: t('help.faq.fee.question'),
    slot: 'faq-fee',
  },
  {
    label: t('help.faq.mirror.question'),
    slot: 'faq-mirror',
  },
]);

/** 点击目录触发程序化滚动期间，暂停滚动监听，避免平滑滚动途中高亮抖动。 */
let isProgrammaticScroll = false;
let scrollTimer: ReturnType<typeof setTimeout> | undefined;

/** 更新当前高亮分类：取顶部越过阈值、最靠下的分类。 */
function updateActiveSection(): void {
  if (isProgrammaticScroll) {
    return;
  }
  const offset = 120;
  let current: HelpSectionId = helpSections[0].id;
  for (const section of helpSections) {
    const el = document.getElementById(section.id);
    if (el !== null && el.getBoundingClientRect().top <= offset) {
      current = section.id;
    }
  }
  activeHelpSectionId.value = current;
}

/** 点击目录跳转到对应分类并立即高亮。 */
function scrollToSection(id: HelpSectionId): void {
  activeHelpSectionId.value = id;
  isProgrammaticScroll = true;
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  clearTimeout(scrollTimer);
  scrollTimer = setTimeout(() => {
    isProgrammaticScroll = false;
  }, 700);
}
watch(helpSectionRequest, (request) => {
  if (request) scrollToSection(request.id);
});
onMounted(() => {
  scrollContainer = helpPage.value?.closest('main') ?? null;
  scrollContainer?.addEventListener('scroll', updateActiveSection, { passive: true });
  updateActiveSection();
});
onBeforeUnmount(() => {
  clearTimeout(scrollTimer);
  scrollContainer?.removeEventListener('scroll', updateActiveSection);
});
</script>

<template>
  <div ref="helpPage">
    <UPageBody class="my-0! space-y-2">
      <!-- 使用前准备 -->
      <WorkspaceSection id="getting-started" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-rocket" />
            <span class="font-semibold text-highlighted">{{ t('help.start.preparation') }}</span>
          </div>
        </template>
        <ol class="flex flex-col gap-3 leading-relaxed text-default">
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">1.</span
            ><span
              ><TranslationComponent :translation="t('help.start.setup')"
                ><template #resolution><strong class="text-primary">1280 × 720</strong></template
                ><template #windowed
                  ><strong class="text-primary">{{ t('help.windowed') }}</strong></template
                ><template #language
                  ><strong class="text-primary">{{
                    t('settings.language.game.fixed')
                  }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">2.</span
            ><span
              ><TranslationComponent :translation="t('help.start.closeOverlay')"
                ><template #hdr
                  ><strong class="text-primary">{{ t('help.hdr.disable') }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
        </ol>
        <p class="mt-3 text-sm text-muted">
          <TranslationComponent :translation="t('help.start.recommended')"
            ><template #ratio><strong>16:9</strong></template
            ><template #resolution><strong>1280 × 720</strong></template
            ><template #windowed
              ><strong>{{ t('help.windowed') }}</strong></template
            ></TranslationComponent
          >
        </p>
      </WorkspaceSection>

      <!-- 档案查漏补缺 -->
      <WorkspaceSection id="archive-scan" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-scan-line" />
            <span class="font-semibold text-highlighted">{{ t('help.start.archive') }}</span>
          </div>
        </template>
        <ol class="flex flex-col gap-3 leading-relaxed text-default">
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">1.</span
            ><span
              ><TranslationComponent :translation="t('help.start.open')"
                ><template #archive
                  ><strong class="text-primary">{{ t('help.archive.main') }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">2.</span
            ><span
              ><TranslationComponent :translation="t('help.start.scan')"
                ><template #scan
                  ><strong class="text-primary">{{ t('help.scan.start') }}</strong></template
                ><template #key><UKbd>'</UKbd></template></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">3.</span
            ><span
              ><TranslationComponent :translation="t('help.start.export')"
                ><template #export
                  ><strong class="text-primary">{{ t('help.export') }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
        </ol>
        <p class="mt-3 text-sm text-muted">
          <TranslationComponent :translation="t('help.start.tip')"
            ><template #archive
              ><strong>{{ t('help.archive.main') }}</strong></template
            ></TranslationComponent
          >
        </p>
      </WorkspaceSection>

      <!-- 百趣演武自动抽 -->
      <WorkspaceSection id="gamble-battle" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-dices" />
            <span class="font-semibold text-highlighted">{{ t('help.start.gambleBattle') }}</span>
          </div>
        </template>
        <ol class="flex flex-col gap-3 leading-relaxed text-default">
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">1.</span
            ><span
              ><TranslationComponent :translation="t('help.gambleBattle.open')"
                ><template #screen
                  ><strong class="text-primary">{{
                    t('help.gambleBattle.screen')
                  }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">2.</span
            ><span
              ><TranslationComponent :translation="t('help.gambleBattle.conditions')"
                ><template #targetHands
                  ><strong class="text-primary">{{
                    t('help.gambleBattle.targetHands')
                  }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">3.</span
            ><span
              ><TranslationComponent :translation="t('help.gambleBattle.start')"
                ><template #start
                  ><strong class="text-primary">{{ t('gambleBattle.start') }}</strong></template
                ><template #key><UKbd>;</UKbd></template></TranslationComponent
              ></span
            >
          </li>
          <li class="flex items-baseline gap-3">
            <span class="w-6 flex-none text-end text-primary tabular-nums">4.</span
            ><span
              ><TranslationComponent :translation="t('help.gambleBattle.stop')"
                ><template #any
                  ><strong class="text-primary">{{ t('help.gambleBattle.any') }}</strong></template
                ></TranslationComponent
              ></span
            >
          </li>
        </ol>
      </WorkspaceSection>

      <!-- 快捷键 -->
      <WorkspaceSection id="usage" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-keyboard" />
            <span class="font-semibold text-highlighted">{{ t('help.usage.title') }}</span>
          </div>
        </template>
        <ul class="flex list-disc flex-col gap-3 ps-6 leading-relaxed marker:text-toned">
          <li>
            <TranslationComponent :translation="t('help.shortcut.scan')"
              ><template #key><UKbd>'</UKbd></template></TranslationComponent
            >
          </li>
          <li>
            <TranslationComponent :translation="t('help.shortcut.gambleBattle')"
              ><template #key><UKbd>;</UKbd></template></TranslationComponent
            >
          </li>
          <li>
            <TranslationComponent :translation="t('help.shortcut.quit')"
              ><template #keys
                ><span><UKbd>Alt</UKbd> + <UKbd>Delete</UKbd></span></template
              ></TranslationComponent
            >
          </li>
        </ul>
      </WorkspaceSection>

      <!-- 常见问题 -->
      <WorkspaceSection id="faq" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-circle-help" />
            <span class="font-semibold text-highlighted">{{ t('help.faq.title') }}</span>
          </div>
        </template>
        <UAccordion :items="faqItems" :ui="{ trigger: 'text-base' }">
          <template #faq-mobile>
            <p class="pb-3.5 text-toned">{{ t('help.faq.mobile.answer') }}</p>
          </template>
          <template #faq-accuracy>
            <p class="pb-3.5 text-toned">
              {{ t('help.faq.accuracy.answer') }}
            </p>
          </template>
          <template #faq-fee
            ><p class="pb-3.5 text-toned">{{ t('help.faq.fee.answer') }}</p></template
          >
          <template #faq-mirror
            ><div class="space-y-2 pb-3.5 text-toned">
              <p>
                <TranslationComponent :translation="t('help.faq.mirror.service')"
                  ><template #service
                    ><ULink
                      class="text-primary hover:text-primary/75"
                      rel="noopener noreferrer"
                      target="_blank"
                      to="https://mirrorchyan.com/"
                      >{{ t('common.mirrorchyan') }}</ULink
                    ></template
                  ></TranslationComponent
                >
              </p>
              <p>
                <TranslationComponent :translation="t('settings.update.mirror.free')"
                  ><template #releases
                    ><ULink
                      class="text-primary hover:text-primary/75"
                      rel="noopener noreferrer"
                      target="_blank"
                      to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                      >GitHub Releases</ULink
                    ></template
                  ></TranslationComponent
                >
              </p>
            </div></template
          >
        </UAccordion>
      </WorkspaceSection>

      <!-- 反馈交流 -->
      <WorkspaceSection id="feedback" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-message-circle" />
            <span class="font-semibold text-highlighted">{{ t('help.feedback.title') }}</span>
          </div>
        </template>
        <div class="flex flex-col gap-4">
          <div class="flex flex-wrap gap-2">
            <UButton
              icon="i-simple-icons:qq"
              :label="t('help.feedback.group')"
              rel="noopener noreferrer"
              target="_blank"
              to="https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms"
            />
            <UButton
              color="neutral"
              icon="i-simple-icons:github"
              :label="t('help.feedback.issue')"
              rel="noopener noreferrer"
              target="_blank"
              to="https://github.com/Logical-Byte/open-endfield-assistant/issues"
            />
          </div>
          <p>
            <TranslationComponent :translation="t('help.feedback.logs')"
              ><template #directory><code>logs/</code></template></TranslationComponent
            >
          </p>
        </div>
      </WorkspaceSection>

      <!-- 致谢 -->
      <WorkspaceSection id="credits" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-heart" />
            <span class="font-semibold text-highlighted">{{ t('help.credits.title') }}</span>
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
            :label="t('help.credits.endfieldMap')"
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
            :label="t('common.mirrorchyan')"
            rel="noopener noreferrer"
            target="_blank"
            to="https://mirrorchyan.com/"
            trailing-icon="i-lucide-external-link"
            variant="link"
          />
          <UButton
            class="px-0"
            color="primary"
            label="RapidAI/RapidOCR"
            rel="noopener noreferrer"
            target="_blank"
            to="https://github.com/RapidAI/RapidOCR"
            trailing-icon="i-simple-icons:github"
            variant="link"
          />
          <UButton
            class="px-0"
            color="primary"
            :label="t('help.credits.rapidOcrModels')"
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
      </WorkspaceSection>

      <!-- 说明 -->
      <WorkspaceSection id="disclaimer" class="scroll-mt-2">
        <template #header>
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-file-text" />
            <span class="font-semibold text-highlighted">{{ t('help.disclaimer.title') }}</span>
          </div>
        </template>
        <ol class="list-decimal space-y-3 ps-6 text-toned marker:text-toned">
          <li>{{ t('help.disclaimer.updates') }}</li>
          <li>{{ t('help.disclaimer.recognition') }}</li>
          <li>{{ t('help.disclaimer.warranty') }}</li>
          <li>{{ t('help.disclaimer.compliance') }}</li>
          <li>{{ t('help.disclaimer.risk') }}</li>
          <li>{{ t('help.disclaimer.acceptance') }}</li>
        </ol>
      </WorkspaceSection>
    </UPageBody>
    <AppFooter />
  </div>
</template>
