<script setup lang="ts">
// 仅供本地原型评估。使用真实设置 store 与内存 IO，不读取或修改用户设置文件。
import { computed, ref, shallowRef, watch } from 'vue';
import { i18n, useAppI18n } from '@/shared/i18n';
import { createSettingsStore, type DraftSettings } from '@/features/settings/settingsStore';
import type { Settings } from '@/features/settings/types/settings';
import SettingsItem from '@/components/settings/SettingsItem.vue';
import SettingsCard from '@/components/settings/SettingsCard.vue';
import SettingsControl from '@/components/settings/SettingsControl.vue';
import { useSettingsFields } from '@/components/settings/useSettingsFields';
import type {
  SettingsControl as Control,
  SettingsItemState,
} from '@/components/settings/settingsItem';

const { t, n, locale } = useAppI18n();
type Scenario = 'ready' | 'loading' | 'initialize-error' | 'unsupported' | 'unreadable';
const scenario = ref<Scenario>('ready');
const saveFails = ref(false);
const saving = ref(false);
const checking = ref(false);
const checkCount = ref(0);
const localScale = ref(1);
const savedPatches = ref<Settings[]>([]);

function makeStore(mode: Scenario): ReturnType<typeof createSettingsStore> {
  const initial: Settings = {
    uiLocale: locale.value === 'zh-CN' ? 'zh-CN' : 'en-US',
    majorVersion: 1,
    minorVersion: 0,
    minimizeToTray: false,
    soundVolume: 0.5,
    updateSource: 'mirrorchyan',
    mirrorchyanCdkEncrypted: mode === 'unreadable' ? 'unreadable' : '',
    updateProxyMode: 'system',
    updateProxyUrl: '',
    autoDownloadUpdates: true,
    autoInstallUpdates: true,
    scanTipsDismissedVersion: 0,
  };
  return createSettingsStore({
    async load(): Promise<Settings> {
      if (mode === 'loading') return new Promise<Settings>(() => {});
      if (mode === 'initialize-error') throw new Error('Prototype load failed');
      return initial;
    },
    async save(values: Settings): Promise<void> {
      saving.value = true;
      try {
        await new Promise<void>((resolve) => window.setTimeout(resolve, 600));
        if (saveFails.value) throw new Error('Prototype save failed');
        savedPatches.value.push(values);
      } finally {
        saving.value = false;
      }
    },
    async encrypt(value: string): Promise<string> {
      return value;
    },
    async decrypt(): Promise<string> {
      throw new Error('Prototype decrypt failed');
    },
  });
}

const store = shallowRef(makeStore('ready'));
void store.value.initialize();
const state = computed(() => store.value.state.value);
const ready = computed(() => (state.value.status === 'ready' ? state.value : null));
const draft = computed(() => ready.value?.draft ?? null);
const unsupported = computed(
  () => state.value.status === 'unavailable' && state.value.reason.type === 'unsupported',
);
const fields = useSettingsFields<DraftSettings>(draft, (patch) => store.value.edit(patch));
const controls = {
  language: fields.choices('uiLocale', [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en-US', label: 'English' },
  ]),
  tray: fields.switch('minimizeToTray'),
  guide: fields.switch('scanGuideEnabled'),
  volume: fields.slider('soundVolume', {
    min: 0,
    max: 1,
    step: 0.05,
    formatValue: (value: number) => n(value, 'percent'),
  }),
  source: fields.select('updateSource', () => [
    { value: 'mirrorchyan', label: t('settings.update.source.mirror') },
    { value: 'oem', label: 'OEM' },
    { value: 'github', label: 'GitHub' },
  ]),
  proxy: fields.select('updateProxyMode', () => [
    { value: 'none', label: t('settings.update.proxy.none') },
    { value: 'system', label: t('settings.update.proxy.system') },
    { value: 'custom', label: t('settings.update.proxy.custom') },
  ]),
  proxyUrl: fields.input('updateProxyUrl', { placeholder: 'http://127.0.0.1:7890' }),
  cdk: fields.input('mirrorchyanCdk', {
    type: 'password',
    placeholder: () => t('settings.update.cdk.placeholder'),
  }),
  scale: {
    kind: 'slider',
    read: () => localScale.value,
    write: (value: number) => {
      localScale.value = value;
    },
    min: 0.5,
    max: 2,
    step: 0.05,
    marks: [0.5, 1, 1.5, 2],
    tooltip: true,
    formatValue: (value: number) => n(value, 'percent'),
  },
  check: {
    kind: 'button',
    label: () => t('settings.update.check.button'),
    icon: 'i-lucide-refresh-cw',
    disabled: () => checking.value,
    loading: () => checking.value,
    onClick: checkUpdate,
  },
} satisfies Record<string, Control>;

const itemState = computed<SettingsItemState>(() => {
  if (state.value.status === 'initializing') return { status: 'loading' };
  return ready.value
    ? { status: 'ready' }
    : {
        status: 'unavailable',
        label: t(unsupported.value ? 'settings.unsupported.label' : 'settings.unavailable'),
      };
});
const languageState = computed<SettingsItemState>(() => ({
  status: ready.value ? 'ready' : 'disabled',
}));
const scaleState = computed<SettingsItemState>(() =>
  unsupported.value
    ? { status: 'unavailable', label: t('settings.unsupported.label'), variant: 'subtle' }
    : { status: 'ready' },
);
const checkState = computed<SettingsItemState>(() => {
  if (state.value.status === 'unavailable' && !unsupported.value) return { status: 'ready' };
  return itemState.value;
});
const snapshot = computed(() => ({
  status: state.value.status,
  draft: ready.value?.draft,
  effective: ready.value?.effective,
  saveError: ready.value?.saveError?.message ?? null,
  saving: saving.value,
  localScale: localScale.value,
  checking: checking.value,
  checkCount: checkCount.value,
  successfulSaves: savedPatches.value.length,
}));
const scenarios = computed<{ value: Scenario; label: string }[]>(() => [
  { value: 'ready', label: t('settingsPrototype.ready') },
  { value: 'loading', label: t('settingsPrototype.loading') },
  { value: 'initialize-error', label: t('settingsPrototype.initializeError') },
  { value: 'unsupported', label: t('settingsPrototype.unsupported') },
  { value: 'unreadable', label: t('settingsPrototype.unreadable') },
]);

async function changeScenario(mode: Scenario): Promise<void> {
  scenario.value = mode;
  store.value = makeStore(mode);
  savedPatches.value = [];
  if (mode === 'unsupported') store.value.markUnsupported();
  else await store.value.initialize();
}

async function checkUpdate(): Promise<void> {
  checking.value = true;
  await new Promise<void>((resolve) => window.setTimeout(resolve, 900));
  checkCount.value += 1;
  checking.value = false;
}

watch(
  () => ready.value?.effective.uiLocale,
  (language) => {
    if (language) void i18n.changeLanguage(language);
  },
);
</script>

<template>
  <UContainer class="space-y-6 py-8">
    <UAlert
      :description="t('settingsPrototype.description')"
      icon="i-lucide-flask-conical"
      :title="t('settingsPrototype.title')"
      variant="subtle"
    />
    <div class="flex flex-wrap items-center gap-3">
      <UButton
        v-for="entry in scenarios"
        :key="entry.value"
        :disabled="saving || checking"
        :label="entry.label"
        :variant="scenario === entry.value ? 'solid' : 'outline'"
        @click="changeScenario(entry.value)"
      />
      <UCheckbox v-model="saveFails" :label="t('settingsPrototype.saveFails')" />
      <UButton
        :disabled="!ready?.saveError || saving"
        :label="t('common.retrySave')"
        @click="store.retry"
      />
    </div>
    <div class="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_24rem]">
      <div class="space-y-6">
        <SettingsCard icon="i-lucide-languages" :title="t('settings.language.section')">
          <SettingsItem
            :control="controls.language"
            :description="t('settings.language.application.description')"
            icon="i-lucide-languages"
            :state="languageState"
            :title="t('settings.language.application.title')"
          />
        </SettingsCard>
        <SettingsCard icon="i-lucide-layout-panel-left" :title="t('settings.interface.section')">
          <SettingsItem
            :control="controls.scale"
            :description="t('settings.interface.scale.description')"
            icon="i-lucide-zoom-in"
            :state="scaleState"
            :title="t('settings.interface.scale.title')"
          />
          <SettingsItem
            :control="controls.tray"
            :description="t('settings.interface.tray.description')"
            icon="i-lucide-panel-bottom-close"
            :state="itemState"
            :title="t('settings.interface.tray.title')"
          />
          <SettingsItem
            :control="controls.guide"
            :description="t('settings.interface.guide.description')"
            icon="i-lucide-circle-help"
            :state="itemState"
            :title="t('settings.interface.guide.title')"
          />
        </SettingsCard>
        <SettingsCard icon="i-lucide-headphones" :title="t('settings.sound.section')">
          <SettingsItem
            :control="controls.volume"
            :description="t('settings.sound.volume.description')"
            icon="i-lucide-volume-2"
            :state="itemState"
            :title="t('settings.sound.volume.title')"
          />
        </SettingsCard>
        <SettingsCard icon="i-lucide-download" :title="t('settings.update.section')">
          <SettingsItem
            :control="controls.source"
            :description="t('settings.update.source.description')"
            icon="i-lucide-cloud-download"
            :state="itemState"
            :title="t('settings.update.source.title')"
          />
          <SettingsItem
            icon="i-lucide-key-round"
            skeleton="input"
            :state="itemState"
            :title="t('settings.update.cdk.title')"
          >
            <div class="flex flex-col items-center gap-1">
              <SettingsControl
                :aria-label="t('settings.update.cdk.title')"
                :control="controls.cdk"
              />
              <template v-if="draft?.mirrorchyanCdk === null">
                <p class="max-w-56 text-sm text-warning">
                  {{ t('settings.update.cdk.unreadable') }}
                </p>
                <UButton
                  color="neutral"
                  :label="t('settings.update.cdk.clear')"
                  variant="link"
                  @click="controls.cdk.write('')"
                />
              </template>
            </div>
          </SettingsItem>
          <SettingsItem
            :control="controls.proxy"
            :description="t('settings.update.proxy.description')"
            icon="i-lucide-network"
            :state="itemState"
            :title="t('settings.update.proxy.title')"
          />
          <SettingsItem
            v-if="draft?.updateProxyMode === 'custom'"
            :control="controls.proxyUrl"
            :description="t('settings.update.proxyUrl.description')"
            icon="i-lucide-link"
            :state="itemState"
            :title="t('settings.update.proxyUrl.title')"
          />
          <SettingsItem
            :control="controls.check"
            :description="t('settings.update.check.description')"
            icon="i-lucide-refresh-cw"
            :state="checkState"
            :title="t('settings.update.check.title')"
          />
        </SettingsCard>
      </div>
      <UCard class="xl:sticky xl:top-6">
        <template #header>{{ t('settingsPrototype.snapshot') }}</template>
        <pre class="overflow-auto text-xs text-toned" data-prototype-state>{{
          JSON.stringify(snapshot, null, 2)
        }}</pre>
      </UCard>
    </div>
  </UContainer>
</template>
