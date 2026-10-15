<script setup lang="ts">
import { TranslationComponent } from 'i18next-vue';
import { i18n, useAppI18n } from '@/shared/i18n';
import { isTauri } from '@tauri-apps/api/core';
import AboutOEA from '@/components/settings/AboutOEA.vue';
import DeveloperSettings from '@/components/settings/DeveloperSettings.vue';
import SettingsControl from '@/components/settings/SettingsControl.vue';
import { useSettingsFields, type SettingsFields } from '@/components/settings/useSettingsFields';
import type {
  ChoiceButtonsSettingsItem,
  SettingsChoice,
  SettingsItemState,
} from '@/components/settings/settingsItem';
import type { UiLocale } from '@/shared/types/locale';
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
import { computed, nextTick, onBeforeUnmount, onMounted, useTemplateRef } from 'vue';
import { useRoute, useRouter, type RouteLocationNormalized } from 'vue-router';

const { t, n, locale } = useAppI18n();
const route = useRoute();
const router = useRouter();

/** 成功加载后供设置页读取的草稿、生效值与保存错误。 */
type ReadySettingsState = Extract<SettingsState, { status: 'ready' }>;

/** 设置分类的导航信息，id 与 URL hash 共用。 */
interface SettingsSection {
  /** 分类标识，同时用于直接打开该分类。 */
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

const applicationLanguageChoices: SettingsChoice<UiLocale>[] = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English', badge: { label: 'beta', color: 'success' } },
];
const applicationLanguageItem: ChoiceButtonsSettingsItem = isTauri()
  ? fields.choices('uiLocale', applicationLanguageChoices, { state: languageItemState })
  : {
      // 浏览器预览直接切换界面语言，无需加载或保存后端设置。
      kind: 'choice-buttons',
      items: applicationLanguageChoices,
      read: (): string => locale.value,
      write: (value: string): void => {
        void i18n.changeLanguage(value);
      },
    };

/** 设置分类：桌面侧栏和窄屏选择框共用。 */
const sections = computed<SettingsSection[]>((): SettingsSection[] => [
  { id: 'language', icon: 'i-lucide-languages', title: t('settings.language.section') },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: t('settings.interface.section') },
  { id: 'sound', icon: 'i-lucide-headphones', title: t('settings.sound.section') },
  { id: 'update', icon: 'i-lucide-download', title: t('settings.update.section') },
  { id: 'developer', icon: 'i-lucide-code-2', title: t('settings.developer.section') },
  { id: 'about', icon: 'i-lucide-info', title: t('settings.about.title') },
]);

/** URL 是当前分类的事实来源，支持直接链接与浏览器前进、后退。 */
const activeSectionId = computed<string>((): string => {
  const id = route.hash.slice(1);
  return sections.value.some((section: SettingsSection): boolean => section.id === id)
    ? id
    : 'language';
});
const settingsPage = useTemplateRef('settingsPage');

function resetScroll(): void {
  const element = settingsPage.value?.$el as HTMLElement | undefined;
  element?.closest('main')?.scrollTo({ top: 0, behavior: 'instant' });
}

onMounted(resetScroll);
// 包含重复访问同一分类的导航，例如再次点击页头的语言设置入口。
const stopResetScroll = router.afterEach((to: RouteLocationNormalized) => {
  if (to.path === '/settings') void nextTick(resetScroll);
});
onBeforeUnmount(stopResetScroll);
</script>

<template>
  <UContainer ref="settingsPage" class="min-h-full">
    <UPage>
      <template #left>
        <UPageAside
          :ui="{
            root: 'lg:sticky lg:top-0 lg:max-h-[calc(100lvh-var(--ui-header-height)-var(--ui-title-height))] lg:self-start lg:overflow-y-auto',
          }"
        >
          <nav :aria-label="t('navigation.settings')" class="flex flex-col gap-1">
            <UButton
              v-for="section in sections"
              :key="section.id"
              :active="activeSectionId === section.id"
              :aria-current="activeSectionId === section.id ? 'location' : undefined"
              class="w-full justify-start"
              :color="activeSectionId === section.id ? 'primary' : 'neutral'"
              :icon="section.icon"
              :label="section.title"
              :to="{ hash: `#${section.id}` }"
              :ui="{ label: 'overflow-visible text-start text-clip whitespace-normal' }"
              :variant="activeSectionId === section.id ? 'soft' : 'ghost'"
            />
          </nav>
        </UPageAside>
      </template>

      <UPageBody class="space-y-4" :data-settings-section="activeSectionId">
        <USelect
          :aria-label="t('navigation.settings')"
          class="w-full lg:hidden"
          :icon="
            sections.find((section: SettingsSection): boolean => section.id === activeSectionId)
              ?.icon
          "
          :items="sections"
          label-key="title"
          :model-value="activeSectionId"
          value-key="id"
          @update:model-value="router.push({ hash: `#${$event}` })"
        />
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
        <AboutOEA v-if="activeSectionId === 'about'" />
        <SettingsCard
          v-if="activeSectionId === 'language'"
          icon="i-lucide-languages"
          :title="t('settings.language.section')"
        >
          <SettingsItem
            :description="t('settings.language.application.description')"
            icon="i-lucide-languages"
            :item="applicationLanguageItem"
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
          v-if="activeSectionId === 'interface'"
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
          v-if="activeSectionId === 'sound'"
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
          v-if="activeSectionId === 'update'"
          icon="i-lucide-download"
          :title="t('settings.update.section')"
        >
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

        <DeveloperSettings
          v-if="activeSectionId === 'developer'"
          :unsupported="settingsUnsupported"
        />
      </UPageBody>
    </UPage>
  </UContainer>
</template>
