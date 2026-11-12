<script setup lang="ts">
import { AppVariant } from '@/app/build';
import { appPresentation } from '@/app/presentation';
import { useUpdateAction } from '@/features/update/useUpdateAction';
import { updateCheckState } from '@/features/update/update';
import UpdateActionIcon from '@/components/update/UpdateActionIcon.vue';
import { useAppI18n } from '@/shared/i18n';
import { computed } from 'vue';
import icon from '../../../resources/icons/icon.png';

const { t, locale } = useAppI18n();
const currentVersion = computed<string>((): string =>
  appPresentation.variant === AppVariant.Dev
    ? t('settings.about.dev')
    : `v${appPresentation.version}`,
);
interface AboutLink {
  label: string;
  icon: string;
  to: string;
  color: 'primary' | 'neutral';
  variant: 'solid' | 'outline';
}

const links = computed<AboutLink[]>((): AboutLink[] => [
  {
    label: t('settings.about.website'),
    icon: 'i-lucide-globe',
    to: 'https://ef.yituliu.cn/resources/oea',
    color: 'primary',
    variant: 'solid',
  },
  {
    label: 'GitHub',
    icon: 'i-simple-icons:github',
    to: 'https://github.com/Logical-Byte/open-endfield-assistant',
    color: 'neutral',
    variant: 'solid',
  },
  {
    label: t('settings.about.community'),
    icon: 'i-simple-icons:qq',
    to: 'https://qm.qq.com/cgi-bin/qm/qr?k=khxbEudh62jRo1KzV_ZnnGqM3Ueq6Yms',
    color: 'primary',
    variant: 'solid',
  },
]);

const action = useUpdateAction();
const {
  phase,
  icon: updateIcon,
  spinning,
  progress,
  label,
  tooltip,
  disabledReason,
  activate,
  release: newRelease,
  version: releaseVersion,
} = action;
const releaseUrl = computed<string>(
  (): string =>
    `https://github.com/Logical-Byte/open-endfield-assistant/releases/tag/${encodeURIComponent(newRelease.value?.versionName ?? '')}`,
);
const lastCheckedText = computed<string>((): string => {
  const checkedAt = updateCheckState.value.lastCheckedAt;
  const time =
    checkedAt === null
      ? t('settings.about.notChecked')
      : new Intl.DateTimeFormat(locale.value, {
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        }).format(checkedAt);
  return t('settings.about.lastChecked', { time });
});
</script>

<template>
  <SettingsCard icon="i-lucide-info" :title="t('settings.sections.about')">
    <template #body>
      <div class="grid items-center gap-5 sm:grid-cols-[max-content_minmax(0,1fr)]">
        <div class="flex items-center gap-3">
          <img alt="OEA" class="size-12 shrink-0" :src="icon" />
          <div class="min-w-0">
            <h2 class="text-lg font-semibold text-highlighted">OEA</h2>
            <p class="mt-1 text-sm text-muted">{{ currentVersion }}</p>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-2 sm:justify-end">
          <UButton
            v-for="link in links"
            :key="link.to"
            :color="link.color"
            :icon="link.icon"
            :label="link.label"
            rel="noopener noreferrer"
            target="_blank"
            :to="link.to"
            :variant="link.variant"
          />
          <UTooltip :text="tooltip">
            <span class="inline-flex" :tabindex="disabledReason ? 0 : undefined">
              <UButton
                color="neutral"
                :disabled="Boolean(disabledReason)"
                :label="label"
                variant="outline"
                @click="activate"
                ><template #leading
                  ><UpdateActionIcon
                    :icon="updateIcon"
                    :progress="progress?.progress ?? null"
                    :ready="phase === 'ready'"
                    :spinning="spinning" /></template
              ></UButton>
            </span>
          </UTooltip>
        </div>

        <div
          v-if="newRelease"
          class="flex flex-col items-start justify-between gap-3 rounded-lg border border-default bg-elevated p-4 sm:col-span-2 sm:flex-row sm:items-center sm:gap-6"
        >
          <div>
            <p class="text-xs text-muted">{{ t('update.check.available') }}</p>
            <p class="mt-1 text-[26px] leading-8 font-bold tracking-[-0.7px] text-primary">
              {{ releaseVersion }}
            </p>
            <p class="mt-1 text-xs text-muted">{{ lastCheckedText }}</p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <UButton
              color="neutral"
              :label="t('settings.about.releaseNotes')"
              rel="noopener noreferrer"
              target="_blank"
              :to="releaseUrl"
              trailing-icon="i-lucide-external-link"
              variant="outline"
            />
          </div>
        </div>
      </div>
      <section
        v-if="appPresentation.variant === AppVariant.Dev"
        class="mt-6 border-t border-default pt-5"
      >
        <h3 class="mb-3 flex items-center gap-2 text-sm font-medium text-toned">
          <UIcon class="size-4" name="i-lucide-info" />
          {{ t('settings.about.devRestrictions.title') }}
        </h3>
        <ul class="list-disc space-y-2 pl-5 text-sm text-muted">
          <li>{{ t('settings.about.devRestrictions.updates') }}</li>
          <li>{{ t('settings.about.devRestrictions.automaticUpdates') }}</li>
          <li>{{ t('settings.about.devRestrictions.localInstall') }}</li>
        </ul>
      </section>
    </template>
  </SettingsCard>
</template>
