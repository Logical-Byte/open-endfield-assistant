<script setup lang="ts">
import AppFooter from '@/components/layout/AppFooter.vue';
import {
  proxyModeItems,
  settingsDraft,
  uiScale,
  UpdateProxyMode,
  updateSourceItems,
} from '@/modules/settings';
import { checkUpdate, updateCheckState, updateOperationBusy } from '@/utils/app/update';
import { oeaVersion } from '@/version';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { createUpdateSettingsBuffers } from '../updateSettingsEditor';
import DeveloperSettings from './DeveloperSettings.vue';
import SettingsCard from './SettingsCard.vue';
import SettingsItem from './SettingsItem.vue';

const toast = useToast();
const route = useRoute();
const updateSettingsBuffers = createUpdateSettingsBuffers(settingsDraft);

/** UI 缩放（本地数字中转）。`USlider` 会短暂回写 `[v]` 数组，这里只允许 number 进入 `uiScale`。 */
const uiScaleNumber = computed<number>({
  get(): number {
    return uiScale.value;
  },
  set(value: number) {
    if (typeof value === 'number') {
      uiScale.value = value;
    }
  },
});

/**
 * 音量（本地数字中转）。
 *
 * Nuxt UI 的 USlider 内部把 v-model 当数组处理，拖动时可能先回写 `[v]`（数组）再回写 `v`（数字），
 * USlider 会短暂回写数组；settingsDraft 会拒绝数组、非有限数值，并钳制合法数字范围。
 */
const soundVolume = computed<number>({
  get(): number {
    return settingsDraft.soundVolume;
  },
  set(value: number) {
    // 先避免数组写入，数值归一化由 settingsDraft 统一完成。
    if (typeof value === 'number') {
      settingsDraft.soundVolume = value;
    }
  },
});

/**
 * 档案扫描页是否展示操作提示。持久化版本映射由 settings 内部负责。
 */
const scanGuideEnabled = computed<boolean>({
  get(): boolean {
    return settingsDraft.scanGuideEnabled;
  },
  set(value: boolean) {
    settingsDraft.scanGuideEnabled = value;
  },
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
  { id: 'interface', icon: 'i-lucide-layout-panel-left', title: '界面设置' },
  { id: 'sound', icon: 'i-lucide-headphones', title: '声音设置' },
  { id: 'update', icon: 'i-lucide-download', title: '更新设置' },
  { id: 'developer', icon: 'i-lucide-code-2', title: '开发者选项' },
];

/** 当前高亮的设置分类 id。 */
const activeSectionId = ref<string>('interface');

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

/** 处理设置页锚点，使弹窗的设置按钮能直达更新设置区域。 */
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
});

watch(
  () => route.hash,
  () => {
    void nextTick().then(scrollToHashSection);
  },
);
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
            <div class="flex w-56 items-center gap-2">
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
          </SettingsItem>
          <SettingsItem
            description="点击窗口关闭按钮时隐藏到系统托盘而不是退出，可通过托盘菜单或 Alt+Delete 退出"
            icon="i-lucide-panel-bottom-close"
            title="关闭时最小化到托盘"
          >
            <USwitch v-model="settingsDraft.minimizeToTray" />
          </SettingsItem>
          <SettingsItem
            description="进入档案扫描页时显示操作指引，关闭后若无更新则不再提示，可随时重新开启"
            icon="i-lucide-circle-help"
            title="显示新手操作提示"
          >
            <USwitch v-model="scanGuideEnabled" />
          </SettingsItem>
        </SettingsCard>

        <SettingsCard id="sound" class="scroll-mt-8" icon="i-lucide-headphones" title="声音设置">
          <SettingsItem
            description="扫描开始、完成、失败或被停止时播放提示音"
            icon="i-lucide-volume-2"
            title="扫描提示音音量"
          >
            <div class="flex w-56 items-center gap-2">
              <USlider v-model="soundVolume" class="flex-1" :max="1" :min="0" :step="0.05" />
              <span class="w-10 text-end text-sm tabular-nums">
                {{ Math.round(soundVolume * 100) }}%
              </span>
            </div>
          </SettingsItem>
        </SettingsCard>

        <SettingsCard id="update" class="scroll-mt-8" icon="i-lucide-download" title="更新设置">
          <SettingsItem
            description="选择从哪个源下载新版本"
            icon="i-lucide-cloud-download"
            title="更新源"
          >
            <USelect v-model="settingsDraft.updateSource" class="w-56" :items="updateSourceItems" />
          </SettingsItem>

          <SettingsItem
            description="检查到新版本后自动开始下载"
            icon="i-lucide-cloud-download"
            title="自动下载更新"
          >
            <USwitch v-model="settingsDraft.autoDownloadUpdates" />
          </SettingsItem>

          <SettingsItem
            description="更新包下载完成后自动安装"
            icon="i-lucide-rocket"
            title="自动安装更新"
          >
            <USwitch v-model="settingsDraft.autoInstallUpdates" />
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
            <div class="flex flex-col items-center gap-1">
              <UInput
                v-model="updateSettingsBuffers.mirrorchyanCdk.value"
                class="w-56"
                placeholder="未填写时使用 OEM 下载"
                type="password"
                @blur="updateSettingsBuffers.commitMirrorchyanCdk"
                @keydown="updateSettingsBuffers.handleMirrorchyanCdkKeydown"
              />
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
          </SettingsItem>

          <SettingsItem
            description="下载更新包时使用的代理方式"
            icon="i-lucide-network"
            title="网络代理"
          >
            <USelect v-model="settingsDraft.updateProxyMode" class="w-56" :items="proxyModeItems" />
          </SettingsItem>

          <SettingsItem
            v-if="settingsDraft.updateProxyMode === UpdateProxyMode.Custom"
            description="自定义代理服务器地址，例如 http://127.0.0.1:7890"
            icon="i-lucide-link"
            title="代理地址"
          >
            <UInput
              v-model="updateSettingsBuffers.updateProxyUrl.value"
              class="w-56"
              placeholder="http://127.0.0.1:7890"
              @blur="updateSettingsBuffers.commitUpdateProxyUrl"
              @keydown="updateSettingsBuffers.handleUpdateProxyUrlKeydown"
            />
          </SettingsItem>
          <div>
            <UButton
              block
              :disabled="updateOperationBusy"
              icon="i-lucide-refresh-cw"
              label="检查更新"
              :loading="updateCheckState.status === 'checking'"
              @click="manualCheckUpdate"
            />
          </div>
        </SettingsCard>

        <DeveloperSettings />
      </UPageBody>
    </UPage>
  </UContainer>

  <AppFooter />
</template>
