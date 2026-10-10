import { formats } from './formats';
import { createI18n, useI18n } from 'vue-i18n';
import type { UiLocale } from '@/shared/types/settings';
import zhCn from './locales/zh-CN.json';
import enUs from './locales/en.json';

export type MessageSchema = typeof zhCn;
export type MessageKey = keyof MessageSchema;
export type MessageParams = Record<string, string | number>;

export const i18n = createI18n<[MessageSchema], UiLocale, false>({
  legacy: false,
  locale: 'en-US',
  fallbackLocale: false,
  flatJson: true,
  messages: { 'zh-CN': zhCn, 'en-US': enUs },
  missing(locale, key) {
    console.warn('Missing translation', { locale, key });
    return key;
  },
  ...formats,
});

/** 普通 TS 模块在展示时调用，避免把译文写入业务状态。 */
export function t(key: MessageKey, params: MessageParams = {}): string {
  return i18n.global.t(key, params);
}
/** 组件与 composable 显式选择全局 Composer。 */
export function useAppI18n(): {
  t: typeof t;
  locale: typeof i18n.global.locale;
  d: typeof i18n.global.d;
  n: typeof i18n.global.n;
} {
  const composer = useI18n({ useScope: 'global' });
  return { t, locale: i18n.global.locale, d: composer.d, n: composer.n };
}
