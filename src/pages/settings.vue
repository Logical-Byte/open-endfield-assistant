<script setup lang="ts">
import { useTranslatedToast } from '@/shared/i18n/toast';
import { TranslationComponent } from 'i18next-vue';
import { useAppI18n } from '@/shared/i18n';
import DeveloperSettings from '@/components/settings/DeveloperSettings.vue';
import SettingsControl from '@/components/settings/SettingsControl.vue';
import { useSettingsFields, type SettingsFields } from '@/components/settings/useSettingsFields';
import type {
  ButtonSettingsItem,
  ChoiceButtonsSettingsItem,
  InputSettingsItem,
  SelectSettingsItem,
  SettingsItemState,
  SliderSettingsItem,
  SwitchSettingsItem,
} from '@/components/settings/settingsItem';
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
import { checkUpdate, updateCheckState, updateOperationBusy } from '@/features/update/update';
import { uiScale } from '@/features/appearance/uiScale';
import { updateCapabilities } from '@/features/update/variantPolicy';
import { appVersion } from '@/app/build';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const { t, n } = useAppI18n();
const route = useRoute();
const router = useRouter();

/** 成功加载后供设置页读取的草稿、生效值与保存错误。 */
type ReadySettingsState = Extract<SettingsState, { status: 'ready' }>;

const readySettings = computed<ReadySettingsState | null>(() =>
  settingsState.value.status === 'ready' ? settingsState.value : null,
);
const draftSettings = computed<Readonly<DraftSettings> | null>(
  () => readySettings.value?.draft ?? null,
);
const settingsInitializing = computed<boolean>(() => settingsState.value.status === 'initializing');
const settingsUnsupported = computed<boolean>(
  () =>
    settingsState.value.status === 'unavailable' &&
    settingsState.value.reason.type === 'unsupported',
);
const settingsInitializeError = computed<Error | null>(() =>
  settingsState.value.status === 'unavailable' &&
  settingsState.value.reason.type === 'initialize-error'
    ? settingsState.value.reason.error
    : null,
);
const unavailableLabel = computed<string>(() =>
  settingsUnsupported.value ? t('settings.unsupported.label') : t('settings.unavailable'),
);
const settingsCanCheckUpdate = computed<boolean>(
  () => readySettings.value !== null || settingsInitializeError.value !== null,
);

/** 手动检查更新。 */
async function manualCheckUpdate(): Promise<void> {
  await checkUpdate();
  if (updateCheckState.value.status === 'upToDate') {
    useTranslatedToast().add({ icon: 'i-lucide-check-circle', color: 'success' }, () => ({
      title: t('update.check.upToDate'),
      description: `v${appVersion}`,
    }));
  }
}

/** 普通持久化字段共用加载与不可用规则，正常编辑保持自动保存。 */
const persistedItemState = computed<SettingsItemState>(() => {
  if (settingsInitializing.value) return { status: 'loading' };
  return draftSettings.value
    ? { status: 'ready' }
    : { status: 'unavailable', label: unavailableLabel.value };
});
/** 语言按钮始终保留，设置尚未可用时禁用，不替换成 skeleton 或 badge。 */
const languageItemState = computed<SettingsItemState>(() => ({
  status: draftSettings.value ? 'ready' : 'disabled',
}));
/** 缩放使用独立的本地状态，后端设置加载或加载失败时仍允许调整。 */
const scaleItemState = computed<SettingsItemState>(() =>
  settingsUnsupported.value
    ? { status: 'unavailable', label: t('settings.unsupported.label'), variant: 'subtle' }
    : { status: 'ready' },
);
/** 初始化失败仍允许检查更新，初始化中显示 skeleton，浏览器模式不可用。 */
const checkUpdateItemState = computed<SettingsItemState>(() => {
  if (settingsInitializing.value) return { status: 'loading' };
  return settingsCanCheckUpdate.value
    ? { status: 'ready' }
    : { status: 'unavailable', label: unavailableLabel.value };
});

const fields: SettingsFields<DraftSettings> = useSettingsFields(draftSettings, editSettings, {
  state: persistedItemState,
});

// 语言：覆盖普通字段的展示状态，保留原有按钮样式。
const applicationLanguageItem: ChoiceButtonsSettingsItem = fields.choices(
  'uiLocale',
  [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en-US', label: 'English' },
  ],
  { state: languageItemState },
);
const gameLanguageItem: ButtonSettingsItem = {
  kind: 'button',
  state: languageItemState,
  label: '简体中文',
  variant: 'solid',
};

// 界面：本地缩放直接连接自己的读写入口，其他项通过字段工厂生成。
const interfaceScaleItem: SliderSettingsItem = {
  kind: 'slider',
  state: scaleItemState,
  read: (): number => uiScale.value,
  write: (value: number): void => {
    uiScale.value = value;
  },
  min: 0.5,
  max: 2,
  step: 0.05,
  marks: [0.5, 1, 1.5, 2],
  tooltip: true,
  formatValue: (value: number): string => n(value, 'percent'),
};
const minimizeToTrayItem: SwitchSettingsItem = fields.switch('minimizeToTray');
const scanGuideItem: SwitchSettingsItem = fields.switch('scanGuideEnabled');

// 声音：仅声明编辑范围和显示格式，数值规范化继续由 settings store 处理。
const soundVolumeItem: SliderSettingsItem = fields.slider('soundVolume', {
  min: 0,
  max: 1,
  step: 0.05,
  formatValue: (value: number): string => n(value, 'percent'),
});

// 更新：持久化字段复用默认状态，检查更新按钮独立声明可用性与忙碌状态。
const updateSourceItem: SelectSettingsItem = fields.select('updateSource', updateSourceItems);
const autoDownloadItem: SwitchSettingsItem = fields.switch('autoDownloadUpdates');
const autoInstallItem: SwitchSettingsItem = fields.switch('autoInstallUpdates');
const mirrorchyanCdkItem: InputSettingsItem = fields.input('mirrorchyanCdk', {
  type: 'password',
  placeholder: (): string => t('settings.update.cdk.placeholder'),
});
const updateProxyItem: SelectSettingsItem = fields.select('updateProxyMode', proxyModeItems);
const updateProxyUrlItem: InputSettingsItem = fields.input('updateProxyUrl', {
  placeholder: 'http://127.0.0.1:7890',
});
const checkUpdateItem: ButtonSettingsItem = {
  kind: 'button',
  state: checkUpdateItemState,
  label: (): string => t('settings.update.check.button'),
  icon: 'i-lucide-refresh-cw',
  onClick: manualCheckUpdate,
  disabled: (): boolean => updateOperationBusy.value,
  loading: (): boolean => updateCheckState.value.status === 'checking',
};

/** 设置分类目录：`id` 同时用作滚动锚点。 */
const sections = computed(() => [
  { id: 'language', icon: 'i-lucide-languages', title: t('settings.language.section') },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: t('settings.interface.section') },
  { id: 'sound', icon: 'i-lucide-headphones', title: t('settings.sound.section') },
  { id: 'update', icon: 'i-lucide-download', title: t('settings.update.section') },
  { id: 'developer', icon: 'i-lucide-code-2', title: t('settings.developer.section') },
]);

/** 当前高亮的设置分类 id。 */
const activeSectionId = ref<string>('language');

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
  if (sections.value.some((section) => section.id === sectionId)) {
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
const stopScrollToHash = router.afterEach((to) => {
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
            :item="applicationLanguageItem"
            :title="t('settings.language.application.title')"
          />
          <div>
            <SettingsItem
              :description="t('settings.language.game.description')"
              icon="i-lucide-gamepad-2"
              :item="gameLanguageItem"
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
            :item="interfaceScaleItem"
            :title="t('settings.interface.scale.title')"
          />
          <SettingsItem
            :description="t('settings.interface.tray.description')"
            icon="i-lucide-panel-bottom-close"
            :item="minimizeToTrayItem"
            :title="t('settings.interface.tray.title')"
          />
          <SettingsItem
            :description="t('settings.interface.guide.description')"
            icon="i-lucide-circle-help"
            :item="scanGuideItem"
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
            :item="soundVolumeItem"
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
          <template v-else>
            <SettingsItem
              :description="t('settings.update.source.description')"
              icon="i-lucide-cloud-download"
              :item="updateSourceItem"
              :title="t('settings.update.source.title')"
            />

            <SettingsItem
              :description="t('settings.update.autoDownload.description')"
              icon="i-lucide-cloud-download"
              :item="autoDownloadItem"
              :title="t('settings.update.autoDownload.title')"
            />

            <SettingsItem
              :description="t('settings.update.autoInstall.description')"
              icon="i-lucide-rocket"
              :item="autoInstallItem"
              :title="t('settings.update.autoInstall.title')"
            />

            <SettingsItem
              icon="i-lucide-key-round"
              :item="mirrorchyanCdkItem"
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
              <div class="flex flex-col items-center gap-1">
                <SettingsControl
                  :aria-label="t('settings.update.cdk.title')"
                  :item="mirrorchyanCdkItem"
                />
                <template v-if="draftSettings?.mirrorchyanCdk === null">
                  <p class="max-w-56 text-sm text-warning">
                    {{ t('settings.update.cdk.unreadable') }}
                  </p>
                  <UButton
                    color="neutral"
                    :label="t('settings.update.cdk.clear')"
                    variant="link"
                    @click="mirrorchyanCdkItem.write('')"
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
            </SettingsItem>

            <SettingsItem
              :description="t('settings.update.proxy.description')"
              icon="i-lucide-network"
              :item="updateProxyItem"
              :title="t('settings.update.proxy.title')"
            />

            <SettingsItem
              v-if="draftSettings?.updateProxyMode === 'custom'"
              :description="t('settings.update.proxyUrl.description')"
              icon="i-lucide-link"
              :item="updateProxyUrlItem"
              :title="t('settings.update.proxyUrl.title')"
            />
            <SettingsItem
              :description="t('settings.update.check.description')"
              icon="i-lucide-refresh-cw"
              :item="checkUpdateItem"
              :title="t('settings.update.check.title')"
            />
          </template>
        </SettingsCard>

        <DeveloperSettings :unsupported="settingsUnsupported" />
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
