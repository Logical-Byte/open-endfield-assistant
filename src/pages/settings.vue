<script setup lang="ts">
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

const toast = useToast();
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
  settingsUnsupported.value ? '浏览器中不可用' : '设置暂不可用',
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
    toast.add({
      title: '当前已是最新版本',
      description: `v${oeaVersion}`,
      icon: 'i-lucide-check-circle',
      color: 'success',
    });
  }
}

/** 设置分类目录：`id` 同时用作滚动锚点。 */
const sections = [
  { id: 'language', icon: 'i-lucide-languages', title: '语言设置' },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: '界面设置' },
  { id: 'sound', icon: 'i-lucide-headphones', title: '声音设置' },
  { id: 'update', icon: 'i-lucide-download', title: '更新设置' },
  { id: 'developer', icon: 'i-lucide-code-2', title: '开发者选项' },
];

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

/** 支持从更新弹窗直接定位到更新设置，也支持在本页再次点击该入口。 */
function scrollToHashSection(): void {
  const sectionId = route.hash.slice(1);
  if (sections.some((section) => section.id === sectionId)) {
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
              :variant="activeSectionId === section.id ? 'soft' : 'ghost'"
              @click="scrollToSection(section.id)"
            />
          </nav>
        </UPageAside>
      </template>

      <UPageBody>
        <UAlert
          v-if="settingsInitializeError"
          :actions="[{ label: '重新初始化', onClick: initOeaSettings }]"
          color="error"
          description="无法从后端初始化设置。界面缩放和开发者功能仍可使用。"
          icon="i-lucide-circle-alert"
          title="设置初始化失败"
          variant="subtle"
        />
        <UAlert
          v-else-if="settingsUnsupported"
          color="neutral"
          description="纯浏览器模式只用于预览页面壳。请运行桌面开发模式来读取和修改设置。"
          icon="i-lucide-monitor-off"
          title="浏览器模式不支持应用设置"
          variant="subtle"
        />
        <UAlert
          v-if="settingsSaveError"
          :actions="[{ label: '重试保存', onClick: retrySettingsSave }]"
          color="error"
          description="已保留当前编辑，应用仍使用最近一次成功保存的设置。"
          title="设置未保存"
        />
        <SettingsCard id="language" class="scroll-mt-8" icon="i-lucide-languages" title="语言设置">
          <SettingsItem
            description="OEA 应用界面使用的语言"
            icon="i-lucide-languages"
            title="应用语言"
          >
            <div class="flex shrink-0 flex-wrap gap-2">
              <UButton
                :disabled="!draftSettings"
                label="简体中文"
                :variant="draftSettings?.uiLocale === 'zh-CN' ? 'solid' : 'outline'"
                @click="editSettings({ uiLocale: 'zh-CN' })"
              />
            </div>
          </SettingsItem>
          <div>
            <SettingsItem
              description="设置为与终末地游戏中使用的语言一致。这会影响自动化任务的运行，也会影响 OEA 应用中显示的部分自动化结果。"
              icon="i-lucide-gamepad-2"
              title="游戏语言"
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
              description="开发中：目前仅支持简体中文"
              icon="i-lucide-triangle-alert"
              variant="subtle"
            />
          </div>
        </SettingsCard>
        <SettingsCard
          id="interface"
          class="scroll-mt-8"
          icon="i-lucide-layout-panel-left"
          title="界面设置"
        >
          <SettingsItem
            description="设置应用窗口的缩放比例，影响所有界面元素的大小"
            icon="i-lucide-zoom-in"
            title="缩放比例"
          >
            <div v-if="!settingsUnsupported" class="flex w-56 items-center gap-2">
              <div class="flex-1">
                <USlider v-model="uiScaleNumber" :max="2" :min="0.5" :step="0.05" tooltip />
                <div class="mt-1 flex justify-between text-xs text-dimmed tabular-nums">
                  <span>50%</span>
                  <span>100%</span>
                  <span>150%</span>
                  <span>200%</span>
                </div>
              </div>
              <span class="w-12 shrink-0 text-end text-sm tabular-nums"
                >{{ Math.round(uiScaleNumber * 100) }}%</span
              >
            </div>
            <UBadge v-else color="neutral" label="浏览器中不可用" variant="subtle" />
          </SettingsItem>
          <SettingsItem
            description="点击窗口关闭按钮时隐藏到系统托盘而不是退出，可通过托盘菜单或 Alt+Delete 退出"
            icon="i-lucide-panel-bottom-close"
            title="关闭时最小化到托盘"
          >
            <USwitch
              v-if="draftSettings"
              :model-value="draftSettings.minimizeToTray"
              @update:model-value="editSettings({ minimizeToTray: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
          <SettingsItem
            description="进入档案扫描页时显示操作指引，关闭后若无更新则不再提示，可随时重新开启"
            icon="i-lucide-circle-help"
            title="显示新手操作提示"
          >
            <USwitch
              v-if="draftSettings"
              :model-value="draftSettings.scanGuideEnabled"
              @update:model-value="editSettings({ scanGuideEnabled: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
        </SettingsCard>

        <SettingsCard id="sound" class="scroll-mt-8" icon="i-lucide-headphones" title="声音设置">
          <SettingsItem
            description="扫描开始、完成、失败或被停止时播放提示音"
            icon="i-lucide-volume-2"
            title="扫描提示音音量"
          >
            <div v-if="draftSettings" class="flex w-56 items-center gap-2">
              <USlider
                class="flex-1"
                :max="1"
                :min="0"
                :model-value="draftSettings.soundVolume"
                :step="0.05"
                @update:model-value="updateSoundVolume"
              />
              <span class="w-10 text-end text-sm tabular-nums">
                {{ Math.round(draftSettings.soundVolume * 100) }}%
              </span>
            </div>
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>
        </SettingsCard>

        <SettingsCard id="update" class="scroll-mt-8" icon="i-lucide-download" title="更新设置">
          <SettingsItem
            description="选择从哪个源下载新版本"
            icon="i-lucide-cloud-download"
            title="更新源"
          >
            <USelect
              v-if="draftSettings"
              class="w-56"
              :items="updateSourceItems"
              :model-value="draftSettings.updateSource"
              @update:model-value="editSettings({ updateSource: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-8 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            description="检查到新版本后自动开始下载"
            icon="i-lucide-cloud-download"
            title="自动下载更新"
          >
            <USwitch
              v-if="draftSettings"
              :model-value="draftSettings.autoDownloadUpdates"
              @update:model-value="editSettings({ autoDownloadUpdates: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            description="更新包下载完成后自动安装"
            icon="i-lucide-rocket"
            title="自动安装更新"
          >
            <USwitch
              v-if="draftSettings"
              :model-value="draftSettings.autoInstallUpdates"
              @update:model-value="editSettings({ autoInstallUpdates: $event })"
            />
            <USkeleton v-else-if="settingsInitializing" class="h-5 w-10 rounded-full" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem icon="i-lucide-key-round" title="Mirror酱 CDK">
            <template #description>
              <span class="text-sm text-dimmed"
                ><ULink class="text-primary hover:text-primary/75" to="https://mirrorchyan.com/"
                  >Mirror酱</ULink
                >
                是独立的第三方加速下载服务，需要付费使用。
                <br />
                OEA 本身不收取任何费用，也提供免费的下载渠道。您可以前往
                <ULink
                  class="text-primary hover:text-primary/75"
                  rel="noopener noreferrer"
                  target="_blank"
                  to="https://github.com/Logical-Byte/open-endfield-assistant/releases"
                  >GitHub Release</ULink
                >
                免费下载和使用。</span
              >
            </template>
            <div v-if="draftSettings" class="flex flex-col items-center gap-1">
              <UInput
                v-model.lazy="mirrorchyanCdk"
                class="w-56"
                placeholder="未填写时使用 OEM 下载"
                type="password"
                @keydown.enter="($event.target as HTMLInputElement).blur()"
              />
              <template v-if="draftSettings.mirrorchyanCdk === null">
                <p class="max-w-56 text-sm text-warning">
                  已保存的 CDK 无法解密，可重新输入或清除。
                </p>
                <UButton
                  color="neutral"
                  label="清除已保存 CDK"
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
                  >没有 CDK？立即订阅<UIcon name="i-lucide-external-link"
                /></span>
              </ULink>
            </div>
            <USkeleton v-else-if="settingsInitializing" class="h-8 w-56" />
            <UBadge v-else color="neutral" :label="unavailableLabel" variant="soft" />
          </SettingsItem>

          <SettingsItem
            description="下载更新包时使用的代理方式"
            icon="i-lucide-network"
            title="网络代理"
          >
            <USelect
              v-if="draftSettings"
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
            description="自定义代理服务器地址，例如 http://127.0.0.1:7890"
            icon="i-lucide-link"
            title="代理地址"
          >
            <UInput
              v-model.lazy="updateProxyUrl"
              class="w-56"
              placeholder="http://127.0.0.1:7890"
              @keydown.enter="($event.target as HTMLInputElement).blur()"
            />
          </SettingsItem>
          <SettingsItem
            description="通过更新服务检查是否存在可用更新"
            icon="i-lucide-refresh-cw"
            title="手动检查更新"
          >
            <USkeleton v-if="settingsInitializing" class="h-8 w-32" />
            <UButton
              v-else-if="settingsCanCheckUpdate"
              :disabled="updateOperationBusy"
              icon="i-lucide-refresh-cw"
              label="检查更新"
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
