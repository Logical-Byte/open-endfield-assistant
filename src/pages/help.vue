<script setup lang="ts">
import { useAppI18n } from '@/shared/i18n';
import { TranslationComponent } from 'i18next-vue';
import type { AccordionItem } from '@nuxt/ui';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

const { t } = useAppI18n();

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

/** 文档目录：`id` 同时用作滚动锚点。 */
const sections = computed(() => [
  { id: 'getting-started', icon: 'i-lucide-rocket', title: t('help.start.title') },
  { id: 'usage', icon: 'i-lucide-keyboard', title: t('help.usage.title') },
  { id: 'known-issues', icon: 'i-lucide-triangle-alert', title: t('help.issues.title') },
  { id: 'faq', icon: 'i-lucide-circle-help', title: t('help.faq.title') },
  { id: 'feedback', icon: 'i-lucide-message-circle', title: t('help.feedback.title') },
  { id: 'credits', icon: 'i-lucide-heart', title: t('help.credits.title') },
  { id: 'disclaimer', icon: 'i-lucide-file-text', title: t('help.disclaimer.title') },
]);

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
  let current = sections.value[0].id;
  for (const section of sections.value) {
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
            root: 'lg:sticky lg:top-0 lg:max-h-[calc(100lvh-var(--ui-header-height)-var(--ui-title-height))] lg:self-start lg:overflow-y-auto',
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
              :ui="{ label: 'overflow-visible text-start text-clip whitespace-normal' }"
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
            :label="t('help.website')"
            rel="noopener noreferrer"
            target="_blank"
            to="https://ef.yituliu.cn/resources/oea"
          />
          <UButton
            color="neutral"
            icon="i-simple-icons:github"
            :label="t('help.repository')"
            rel="noopener noreferrer"
            target="_blank"
            to="https://github.com/Logical-Byte/open-endfield-assistant"
          />
          <UButton
            icon="i-simple-icons:qq"
            :label="t('help.feedback.group')"
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
              <span class="font-semibold text-highlighted">{{ t('help.start.title') }}</span>
            </div>
          </template>
          <ol class="flex flex-col gap-4 text-lg leading-relaxed font-medium">
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">1.</span
              ><span
                ><TranslationComponent :translation="t('help.start.resolution')"
                  ><template #resolution><strong class="text-primary">1280 × 720</strong></template
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
                ><TranslationComponent :translation="t('help.start.hdr')"
                  ><template #hdr
                    ><strong class="text-primary">{{ t('help.hdr.disable') }}</strong></template
                  ></TranslationComponent
                ></span
              >
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">3.</span
              ><span
                ><TranslationComponent :translation="t('help.start.archive')"
                  ><template #archive
                    ><strong class="text-primary">{{ t('help.archive.main') }}</strong></template
                  ></TranslationComponent
                ></span
              >
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">4.</span
              ><span
                ><TranslationComponent :translation="t('help.start.scan')"
                  ><template #scan
                    ><strong class="text-primary">{{ t('help.scan.start') }}</strong></template
                  ></TranslationComponent
                ></span
              >
            </li>
            <li class="flex items-baseline gap-3">
              <span class="w-6 flex-none text-end text-primary tabular-nums">5.</span
              ><span
                ><TranslationComponent :translation="t('help.start.export')"
                  ><template #export
                    ><strong class="text-primary">{{ t('help.export') }}</strong></template
                  ></TranslationComponent
                ></span
              >
            </li>
          </ol>
        </UCard>

        <!-- 操作说明 -->
        <UCard id="usage" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-keyboard" />
              <span class="font-semibold text-highlighted">{{ t('help.usage.title') }}</span>
            </div>
          </template>
          <div class="space-y-6">
            <div>
              <p class="mb-3 font-semibold text-highlighted">{{ t('help.usage.preparation') }}</p>
              <ul class="flex list-disc flex-col gap-3 ps-6 text-toned marker:text-toned">
                <li>
                  <TranslationComponent :translation="t('help.usage.resolution')"
                    ><template #ratio><strong class="text-primary">16:9</strong></template
                    ><template #resolution
                      ><strong class="text-primary">1280 × 720</strong></template
                    ><template #windowed
                      ><strong class="text-primary">{{ t('help.windowed') }}</strong></template
                    ></TranslationComponent
                  >
                </li>
                <li>
                  <TranslationComponent :translation="t('help.usage.archive')"
                    ><template #archive
                      ><strong class="text-primary">{{ t('help.archive.main') }}</strong></template
                    ></TranslationComponent
                  >
                </li>
                <li>
                  <TranslationComponent :translation="t('help.usage.language')"
                    ><template #language
                      ><strong class="text-primary">{{
                        t('settings.language.game.fixed')
                      }}</strong></template
                    ></TranslationComponent
                  >
                </li>
                <li>
                  <TranslationComponent :translation="t('help.usage.hdr')"
                    ><template #hdr
                      ><strong class="text-primary">{{ t('help.hdr.disable') }}</strong></template
                    ></TranslationComponent
                  >
                </li>
              </ul>
            </div>
            <div>
              <p class="mb-3 font-semibold text-highlighted">{{ t('help.usage.shortcuts') }}</p>
              <ul class="flex list-disc flex-col gap-3 ps-6 text-toned marker:text-toned">
                <li>
                  <TranslationComponent :translation="t('help.shortcut.scan')"
                    ><template #key><UKbd>'</UKbd></template></TranslationComponent
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
            </div>
          </div>
        </UCard>

        <!-- 已知问题 -->
        <UCard id="known-issues" class="scroll-mt-8">
          <template #header>
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-triangle-alert" />
              <span class="font-semibold text-highlighted">{{ t('help.issues.title') }}</span>
            </div>
          </template>
          <ol class="list-disc space-y-3 text-toned">
            <li>
              <TranslationComponent :translation="t('help.issues.sharedTitle')"
                ><template #title><span>挂在竹子上的字条</span></template></TranslationComponent
              >
            </li>
          </ol>
        </UCard>

        <!-- 常见问题 -->
        <UCard id="faq" class="scroll-mt-8">
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
                        >MirrorChyan</ULink
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
        </UCard>

        <!-- 反馈交流 -->
        <UCard id="feedback" class="scroll-mt-8">
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
        </UCard>

        <!-- 致谢 -->
        <UCard id="credits" class="scroll-mt-8">
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
              :label="t('help.credits.models')"
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
              <span class="font-semibold text-highlighted">{{ t('help.disclaimer.title') }}</span>
            </div>
          </template>
          <ol class="list-disc space-y-3 ps-6 text-toned marker:text-toned">
            <li>{{ t('help.disclaimer.updates') }}</li>
            <li>{{ t('help.disclaimer.recognition') }}</li>
            <li>{{ t('help.disclaimer.warranty') }}</li>
            <li>{{ t('help.disclaimer.compliance') }}</li>
            <li>{{ t('help.disclaimer.risk') }}</li>
            <li>{{ t('help.disclaimer.acceptance') }}</li>
          </ol>
        </UCard>
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
