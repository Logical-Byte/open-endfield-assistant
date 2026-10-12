import { createInstance, type ParseKeys } from 'i18next';
import { useTranslation } from 'i18next-vue';
import { readonly, shallowRef } from 'vue';
import { dateFormats, numberFormats } from './formats';
import zhCn from './locales/zh-CN.json';
import enUs from './locales/en.json';

export type MessageKey = ParseKeys<'translation'>;
export type MessageParams = Record<string, string | number> & { count?: number };

export const i18n = createInstance();
// 所有资源随应用打包，初始化同步完成，不依赖网络或语言探测。
void i18n.init({
  lng: 'en-US',
  fallbackLng: false,
  load: 'currentOnly',
  initAsync: false,
  enableSelector: false,
  resources: { 'zh-CN': { translation: zhCn }, 'en-US': { translation: enUs } },
  interpolation: { escapeValue: false },
  returnNull: false,
  parseMissingKeyHandler(key): string {
    console.warn('Missing translation', { locale: i18n.language, key });
    return key;
  },
});

const currentLocale = shallowRef(i18n.language);
i18n.on('languageChanged', (language: string) => {
  currentLocale.value = language;
});
export const locale = readonly(currentLocale);

/** 普通 TS 模块在展示时调用，并让 computed 跟踪语言变化。 */
export function t(key: MessageKey, params: MessageParams = {}): string {
  void locale.value;
  return i18n.t(key, params);
}

export function d(value: Date | number, format: keyof typeof dateFormats): string {
  return new Intl.DateTimeFormat(locale.value, dateFormats[format]).format(value);
}

export function n(
  value: number,
  format: keyof typeof numberFormats | Intl.NumberFormatOptions = 'quantity',
): string {
  const options = typeof format === 'string' ? numberFormats[format] : format;
  return new Intl.NumberFormat(locale.value, options).format(value);
}

/** Vue 集成负责组件内的翻译依赖，日期数字共用当前语言。 */
export function useAppI18n(): {
  t: ReturnType<typeof useTranslation<'translation'>>['t'];
  locale: typeof locale;
  d: typeof d;
  n: typeof n;
} {
  const { t } = useTranslation('translation');
  return { t, locale, d, n };
}
