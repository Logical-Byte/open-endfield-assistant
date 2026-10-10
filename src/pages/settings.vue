<script setup lang="ts">
import { useTranslatedToast } from '@/shared/i18n/toast';
import { TranslationComponent } from 'i18next-vue';
import { useAppI18n } from '@/shared/i18n';
import DeveloperSettings from '@/components/settings/DeveloperSettings.vue';
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
import { oeaVersion } from '@/version';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const { t, n } = useAppI18n();
const route = useRoute();
const router = useRouter();

const readySettings = computed(() =>
  settingsState.value.status === 'ready' ? settingsState.value : null,
);
const draftSettings = computed(() => readySettings.value?.draft ?? null);
const settingsInitializing = computed(() => settingsState.value.status === 'initializing');
const settingsUnsupported = computed(
  () =>
    settingsState.value.status === 'unavailable' &&
    settingsState.value.reason.type === 'unsupported',
);
const settingsInitializeError = computed(() =>
  settingsState.value.status === 'unavailable' &&
  settingsState.value.reason.type === 'initialize-error'
    ? settingsState.value.reason.error
    : null,
);
const unavailableLabel = computed(() =>
  settingsUnsupported.value ? t('settings.unsupported.label') : t('settings.unavailable'),
);
const settingsCanCheckUpdate = computed(
  () => readySettings.value !== null || settingsInitializeError.value !== null,
);

/** UI 缩放（本地数字中转）。`USlider` 会短暂回写 `[v]` 数组，这里只允许 number 进入 `uiScale`。 */
const uiScaleNumber = computed<number>({
  get() {
    return uiScale.value;
  },
  set(value: number) {
    if (typeof value === 'number') {
      uiScale.value = value;
    }
  },
});

function updateSoundVolume(value: number | number[] | undefined): void {
  if (typeof value === 'number') {
    editSettings({ soundVolume: value });
  }
}

// UInput 自带 lazy 提交，输入中间值留在控件内。普通失焦不会产生修改。
const mirrorchyanCdk = computed<string>({
  get: () => draftSettings.value?.mirrorchyanCdk ?? '',
  set: (value: string) => editSettings({ mirrorchyanCdk: value }),
});
const updateProxyUrl = computed<string>({
  get: () => draftSettings.value?.updateProxyUrl ?? '',
  set: (value: string) => editSettings({ updateProxyUrl: value }),
});

/** 手动检查更新。 */
async function manualCheckUpdate(): Promise<void> {
  await checkUpdate();
  if (updateCheckState.value.status === 'upToDate') {
    useTranslatedToast().add({ icon: 'i-lucide-check-circle', color: 'success' }, () => ({
      title: t('update.check.upToDate'),
      description: `v${oeaVersion}`,
    }));
  }
}

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
            :title="t('settings.language.application.title')"
          >
            <div class="flex shrink-0 flex-wrap gap-2">
              <UButton
                :disabled="!draftSettings"
                label="简体中文"
                :variant="draftSettings?.uiLocale === 'zh-CN' ? 'solid' : 'outline'"
                @click="editSettings({ uiLocale: 'zh-CN' })"
              />
              <UButton
                :disabled="!draftSettings"
                label="English"
                :variant="draftSettings?.uiLocale === 'en-US' ? 'solid' : 'outline'"
                @click="editSettings({ uiLocale: 'en-US' })"
              />
            </div>
          </SettingsItem>
          <div>
            <SettingsItem
              :description="t('settings.language.game.description')"
              icon="i-lucide-gamepad-2"
              :title="t('settings.language.game.title')"
            >
              <UButton
                class="shrink-0"
                :disabled="!draftSettings"
                label="简体中文"
                variant="solid"
              />
            </SettingsItem>
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
            :title="t('settings.interface.scale.title')"
          >
            <div v-if="!settingsUnsupported" class="flex w-56 items-center gap-2">
              <div class="flex-1">
                <USlider
                  v-model="uiScaleNumber"
                  :aria-label="t('settings.interface.scale.title')"
                  :max="2"
                  :min="0.5"
                  :step="0.05"
                  tooltip
                />
                <div class="mt-1 flex justify-between text-xs text-dimmed tabular-nums">
                  <span>{{ n(0.5, 'percent') }}</span>
                  <span>{{ n(1, 'percent') }}</span>
                  <span>{{ n(1.5, 'percent') }}</span>
                  <span>{{ n(2, 'percent') }}</span>
                </div>
              </div>
              <span class="w-12 shrink-0 text-end text-sm tabular-nums">{{
                n(uiScaleNumber, 'percent')
              }}</span>
            </div>
            <UBadge
              v-else
              color="neutral"
              :label="t('settings.unsupported.label')"
              variant="subtle"
            />
          </SettingsItem>
          <SettingsItem
            :description="t('settings.interface.tray.description')"
            icon="i-lucide-panel-bottom-close"
            :title="t('settings.interface.tray.title')"
          >
            <USwitch
              v-if="draftSettings"
              :aria-label="t('settings.interface.tray.title')"
              :model-value="draftSettings.minimizeToTray"
              @update:model-value="editSettings({ minimizeToTray: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
          <SettingsItem
            :description="t('settings.interface.guide.description')"
            icon="i-lucide-circle-help"
            :title="t('settings.interface.guide.title')"
          >
            <USwitch
              v-if="draftSettings"
              :aria-label="t('settings.interface.guide.title')"
              :model-value="draftSettings.scanGuideEnabled"
              @update:model-value="editSettings({ scanGuideEnabled: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
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
            :title="t('settings.sound.volume.title')"
          >
            <div v-if="draftSettings" class="flex w-56 items-center gap-2">
              <USlider
                :aria-label="t('settings.sound.volume.title')"
                class="flex-1"
                :max="1"
                :min="0"
                :model-value="draftSettings.soundVolume"
                :step="0.05"
                @update:model-value="updateSoundVolume"
              />
              <span class="w-10 text-end text-sm tabular-nums">
                {{ n(draftSettings.soundVolume, 'percent') }}
              </span>
            </div>
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
        </SettingsCard>

        <SettingsCard
          id="update"
          class="scroll-mt-8"
          icon="i-lucide-download"
          :title="t('settings.update.section')"
        >
          <SettingsItem
            :description="t('settings.update.source.description')"
            icon="i-lucide-cloud-download"
            :title="t('settings.update.source.title')"
          >
            <USelect
              v-if="draftSettings"
              :aria-label="t('settings.update.source.title')"
              class="w-56"
              :items="updateSourceItems"
              :model-value="draftSettings.updateSource"
              @update:model-value="editSettings({ updateSource: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-8 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            :description="t('settings.update.autoDownload.description')"
            icon="i-lucide-cloud-download"
            :title="t('settings.update.autoDownload.title')"
          >
            <USwitch
              v-if="draftSettings"
              :aria-label="t('settings.update.autoDownload.title')"
              :model-value="draftSettings.autoDownloadUpdates"
              @update:model-value="editSettings({ autoDownloadUpdates: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            :description="t('settings.update.autoInstall.description')"
            icon="i-lucide-rocket"
            :title="t('settings.update.autoInstall.title')"
          >
            <USwitch
              v-if="draftSettings"
              :aria-label="t('settings.update.autoInstall.title')"
              :model-value="draftSettings.autoInstallUpdates"
              @update:model-value="editSettings({ autoInstallUpdates: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem icon="i-lucide-key-round" :title="t('settings.update.cdk.title')">
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
            <div v-if="draftSettings" class="flex flex-col items-center gap-1">
              <UInput
                v-model.lazy="mirrorchyanCdk"
                :aria-label="t('settings.update.cdk.title')"
                class="w-56"
                :placeholder="t('settings.update.cdk.placeholder')"
                type="password"
                @keydown.enter="($event.target as HTMLInputElement).blur()"
              />
              <template v-if="draftSettings.mirrorchyanCdk === null">
                <p class="max-w-56 text-sm text-warning">
                  {{ t('settings.update.cdk.unreadable') }}
                </p>
                <UButton
                  color="neutral"
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
            <USkeleton v-else-if="settingsInitializing" class="h-8 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            :description="t('settings.update.proxy.description')"
            icon="i-lucide-network"
            :title="t('settings.update.proxy.title')"
          >
            <USelect
              v-if="draftSettings"
              :aria-label="t('settings.update.proxy.title')"
              class="w-56"
              :items="proxyModeItems"
              :model-value="draftSettings.updateProxyMode"
              @update:model-value="editSettings({ updateProxyMode: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-8 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            v-if="draftSettings?.updateProxyMode === 'custom'"
            :description="t('settings.update.proxyUrl.description')"
            icon="i-lucide-link"
            :title="t('settings.update.proxyUrl.title')"
          >
            <UInput
              v-model.lazy="updateProxyUrl"
              :aria-label="t('settings.update.proxyUrl.title')"
              class="w-56"
              placeholder="http://127.0.0.1:7890"
              @keydown.enter="($event.target as HTMLInputElement).blur()"
            />
          </SettingsItem>
          <SettingsItem
            :description="t('settings.update.check.description')"
            icon="i-lucide-refresh-cw"
            :title="t('settings.update.check.title')"
          >
            <USkeleton v-if="settingsInitializing" class="h-8 w-32" />
            <UButton
              v-else-if="settingsCanCheckUpdate"
              :disabled="updateOperationBusy"
              icon="i-lucide-refresh-cw"
              :label="t('settings.update.check.button')"
              :loading="updateCheckState.status === 'checking'"
              @click="manualCheckUpdate"
            />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
        </SettingsCard>

        <DeveloperSettings :unsupported="settingsUnsupported" />
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
