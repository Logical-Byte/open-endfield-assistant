<script setup lang="ts">
import { TranslationComponent } from 'i18next-vue';
import { useAppI18n } from '@/shared/i18n';
import AboutOEA from '@/components/settings/AboutOEA.vue';
import DeveloperSettings from '@/components/settings/DeveloperSettings.vue';
import SettingsControl from '@/components/settings/SettingsControl.vue';
import { useSettingsFields, type SettingsFields } from '@/components/settings/useSettingsFields';
import type { SettingsItemState } from '@/components/settings/settingsItem';
import type { DraftSettings, SettingsState } from '@/features/settings/settingsStore';
import {
  settingsSaveError,
  editSettings,
  initOeaSettings,
  settingsState,
  retrySettingsSave,
  proxyModeItems,
  updateSourceItems,
} from '@/features/settings/settings';
import { uiScale } from '@/features/appearance/uiScale';
import { updateCapabilities } from '@/features/update/variantPolicy';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute, useRouter, type RouteLocationNormalized } from 'vue-router';

const { t, n } = useAppI18n();
const route = useRoute();
const router = useRouter();

/** 成功加载后供设置页读取的草稿、生效值与保存错误。 */
type ReadySettingsState = Extract<SettingsState, { status: 'ready' }>;

/** 设置分类目录的展示信息，与滚动锚点共用 id。 */
interface SettingsSection {
  /** 设置卡片的 DOM id，也是目录导航目标。 */
  id: string;
  /** 目录和卡片共用的完整图标名。 */
  icon: string;
  /** 随应用语言更新的分类标题。 */
  title: string;
}

const readySettings = computed<ReadySettingsState | null>((): ReadySettingsState | null =>
  settingsState.value.status === 'ready' ? settingsState.value : null,
);
const draftSettings = computed<Readonly<DraftSettings> | null>(
  (): Readonly<DraftSettings> | null => readySettings.value?.draft ?? null,
);
const settingsInitializing = computed<boolean>(
  (): boolean => settingsState.value.status === 'initializing',
);
const settingsUnsupported = computed<boolean>(
  (): boolean =>
    settingsState.value.status === 'unavailable' &&
    settingsState.value.reason.type === 'unsupported',
);
const settingsInitializeError = computed<Error | null>((): Error | null =>
  settingsState.value.status === 'unavailable' &&
  settingsState.value.reason.type === 'initialize-error'
    ? settingsState.value.reason.error
    : null,
);
const unavailableLabel = computed<string>((): string =>
  settingsUnsupported.value ? t('settings.unsupported.label') : t('settings.unavailable'),
);
/** 普通持久化字段共用加载与不可用规则，正常编辑保持自动保存。 */
const persistedItemState = computed<SettingsItemState>((): SettingsItemState => {
  if (settingsInitializing.value) return { status: 'loading' };
  return draftSettings.value
    ? { status: 'ready' }
    : { status: 'unavailable', label: unavailableLabel.value };
});
/** 语言按钮始终保留，设置尚未可用时禁用，不替换成 skeleton 或 badge。 */
const languageItemState = computed<SettingsItemState>((): SettingsItemState => ({
  status: draftSettings.value ? 'ready' : 'disabled',
}));
/** 缩放使用独立的本地状态，后端设置加载或加载失败时仍允许调整。 */
const scaleItemState = computed<SettingsItemState>((): SettingsItemState =>
  settingsUnsupported.value
    ? { status: 'unavailable', label: t('settings.unsupported.label'), variant: 'subtle' }
    : { status: 'ready' },
);
const fields: SettingsFields<DraftSettings> = useSettingsFields(draftSettings, editSettings, {
  defaults: { state: persistedItemState },
});

/** 设置分类目录：`id` 同时用作滚动锚点。 */
const sections = computed<SettingsSection[]>((): SettingsSection[] => [
  { id: 'about', icon: 'i-lucide-info', title: t('settings.about.title') },
  { id: 'language', icon: 'i-lucide-languages', title: t('settings.language.section') },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: t('settings.interface.section') },
  { id: 'sound', icon: 'i-lucide-headphones', title: t('settings.sound.section') },
  { id: 'update', icon: 'i-lucide-download', title: t('settings.update.section') },
  { id: 'developer', icon: 'i-lucide-code-2', title: t('settings.developer.section') },
]);

/** 当前高亮的设置分类 id。 */
const activeSectionId = ref<string>('about');

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

/** 支持从更新弹窗直接定位到更新设置，也支持在本页再次点击该入口。 */
function scrollToHashSection(): void {
  const sectionId = route.hash.slice(1);
  if (sections.value.some((section: SettingsSection): boolean => section.id === sectionId)) {
    scrollToSection(sectionId);
  }
}

onMounted(() => {
  // `UMain` 渲染为 <main>，是实际滚动容器。
  document
    .querySelector('main')
    ?.addEventListener('scroll', updateActiveSection, { passive: true });
  void nextTick().then(scrollToHashSection);
});

onBeforeUnmount(() => {
  document.querySelector('main')?.removeEventListener('scroll', updateActiveSection);
  stopScrollToHash();
});

// 相同 hash 的重复导航也会经过 afterEach，滚离更新区域后仍可用齿轮重新定位。
const stopScrollToHash = router.afterEach((to: RouteLocationNormalized) => {
  if (to.path === '/settings') {
    void nextTick().then(scrollToHashSection);
  }
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

      <UPageBody>
        <AboutOEA />
        <UAlert
          v-if="settingsInitializeError"
          :actions="[{ label: t('settings.initialize.retry'), onClick: initOeaSettings }]"
          color="error"
          :description="t('settings.initialize.failed.pageDescription')"
          icon="i-lucide-circle-alert"
          :title="t('settings.initialize.failed.title')"
          variant="subtle"
        />
        <UAlert
          v-else-if="settingsUnsupported"
          color="neutral"
          :description="t('settings.unsupported.description')"
          icon="i-lucide-monitor-off"
          :title="t('settings.unsupported.title')"
          variant="subtle"
        />
        <UAlert
          v-if="settingsSaveError"
          :actions="[{ label: t('common.retrySave'), onClick: retrySettingsSave }]"
          color="error"
          :description="t('settings.save.failed.description')"
          :title="t('settings.save.failed.title')"
        />
        <SettingsCard
          id="language"
          class="scroll-mt-8"
          icon="i-lucide-languages"
          :title="t('settings.language.section')"
        >
          <SettingsItem
            :description="t('settings.language.application.description')"
            icon="i-lucide-languages"
            :item="
              fields.choices(
                'uiLocale',
                [
                  { value: 'zh-CN', label: '简体中文' },
                  { value: 'en-US', label: 'English' },
                ],
                { state: languageItemState },
              )
            "
            :title="t('settings.language.application.title')"
          />
          <div>
            <SettingsItem
              :description="t('settings.language.game.description')"
              icon="i-lucide-gamepad-2"
              :item="{
                kind: 'button',
                state: languageItemState,
                label: '简体中文',
                variant: 'solid',
              }"
              :title="t('settings.language.game.title')"
            />
            <UAlert
              class="mt-4"
              color="warning"
              :description="t('settings.language.game.warning')"
              icon="i-lucide-triangle-alert"
              variant="subtle"
            />
          </div>
        </SettingsCard>
        <SettingsCard
          id="interface"
          class="scroll-mt-8"
          icon="i-lucide-layout-panel-left"
          :title="t('settings.interface.section')"
        >
          <SettingsItem
            :description="t('settings.interface.scale.description')"
            icon="i-lucide-zoom-in"
            :item="{
              kind: 'slider',
              state: scaleItemState,
              read: (): number => uiScale,
              write: (value: number) => {
                uiScale = value;
              },
              min: 0.5,
              max: 2,
              step: 0.05,
              marks: [0.5, 1, 1.5, 2],
              tooltip: true,
              formatValue: (value: number): string => n(value, 'percent'),
            }"
            :title="t('settings.interface.scale.title')"
          />
          <SettingsItem
            :description="t('settings.interface.tray.description')"
            icon="i-lucide-panel-bottom-close"
            :item="fields.switch('minimizeToTray')"
            :title="t('settings.interface.tray.title')"
          />
          <SettingsItem
            :description="t('settings.interface.guide.description')"
            icon="i-lucide-circle-help"
            :item="fields.switch('scanGuideEnabled')"
            :title="t('settings.interface.guide.title')"
          />
        </SettingsCard>

        <SettingsCard
          id="sound"
          class="scroll-mt-8"
          icon="i-lucide-headphones"
          :title="t('settings.sound.section')"
        >
          <SettingsItem
            :description="t('settings.sound.volume.description')"
            icon="i-lucide-volume-2"
            :item="
              fields.slider('soundVolume', {
                min: 0,
                max: 1,
                step: 0.05,
                formatValue: (value: number): string => n(value, 'percent'),
              })
            "
            :title="t('settings.sound.volume.title')"
          />
        </SettingsCard>

        <SettingsCard
          id="update"
          class="scroll-mt-8"
          icon="i-lucide-download"
          :title="t('settings.update.section')"
        >
          <p v-if="!updateCapabilities.onlineUpdate" class="text-sm text-muted">
            {{ t('settings.update.developmentBuild') }}
          </p>
          <SettingsItem
            :description="t('settings.update.source.description')"
            icon="i-lucide-cloud-download"
            :item="fields.select('updateSource', updateSourceItems)"
            :title="t('settings.update.source.title')"
          />

          <SettingsItem
            :description="t('settings.update.autoDownload.description')"
            icon="i-lucide-cloud-download"
            :item="fields.switch('autoDownloadUpdates')"
            :title="t('settings.update.autoDownload.title')"
          />

          <SettingsItem
            :description="t('settings.update.autoInstall.description')"
            icon="i-lucide-rocket"
            :item="fields.switch('autoInstallUpdates')"
            :title="t('settings.update.autoInstall.title')"
          />

          <SettingsItem
            icon="i-lucide-key-round"
            :item="
              fields.input('mirrorchyanCdk', {
                type: 'password',
                placeholder: (): string => t('settings.update.cdk.placeholder'),
              })
            "
            :title="t('settings.update.cdk.title')"
          >
            <template #description>
              <div class="space-y-1 text-sm text-dimmed">
                <p>
                  <TranslationComponent :translation="t('settings.update.mirror.service')">
                    <template #service
                      ><ULink
                        class="text-primary hover:text-primary/75"
                        to="https://mirrorchyan.com/"
                        >MirrorChyan</ULink
                      ></template
                    >
                  </TranslationComponent>
                </p>
                <p>
                  <TranslationComponent :translation="t('settings.update.mirror.free')">
                    <template #releases
                      ><ULink
                        class="text-primary hover:text-primary/75"
                        rel="noopener noreferrer"
                        target="_blank"
                        to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                        >GitHub Releases</ULink
                      ></template
                    >
                  </TranslationComponent>
                </p>
              </div>
            </template>
            <template #default="{ item: cdkItem, disabled }">
              <div class="flex flex-col items-center gap-1">
                <SettingsControl
                  :aria-label="t('settings.update.cdk.title')"
                  :disabled="disabled"
                  :item="cdkItem!"
                />
                <template v-if="draftSettings?.mirrorchyanCdk === null">
                  <p class="max-w-56 text-sm text-warning">
                    {{ t('settings.update.cdk.unreadable') }}
                  </p>
                  <UButton
                    color="neutral"
                    :disabled="disabled"
                    :label="t('settings.update.cdk.clear')"
                    variant="link"
                    @click="editSettings({ mirrorchyanCdk: '' })"
                  />
                </template>
                <ULink
                  class="text-sm text-primary hover:text-primary/75"
                  rel="noopener noreferrer"
                  target="_blank"
                  to="https://mirrorchyan.com/?source=oea"
                >
                  <span class="flex items-center gap-1"
                    >{{ t('settings.update.cdk.subscribe') }}<UIcon name="i-lucide-external-link"
                  /></span>
                </ULink>
              </div>
            </template>
          </SettingsItem>

          <SettingsItem
            :description="t('settings.update.proxy.description')"
            icon="i-lucide-network"
            :item="fields.select('updateProxyMode', proxyModeItems)"
            :title="t('settings.update.proxy.title')"
          />

          <SettingsItem
            v-if="draftSettings?.updateProxyMode === 'custom'"
            :description="t('settings.update.proxyUrl.description')"
            icon="i-lucide-link"
            :item="fields.input('updateProxyUrl', { placeholder: 'http://127.0.0.1:7890' })"
            :title="t('settings.update.proxyUrl.title')"
          />
        </SettingsCard>

        <DeveloperSettings :unsupported="settingsUnsupported" />
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
