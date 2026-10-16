import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import I18NextVue, { TranslationComponent } from 'i18next-vue';
import { computed, createApp, createSSRApp, h, nextTick } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { d, i18n, locale, n, t, useAppI18n } from './index';

// 独立 fixture 验证翻译入口，不覆盖随应用交付的 en-US 资源。
const englishFixture = {
  settings: { save: { failed: { title: 'Settings not saved' } } },
  scan: {
    matches_one: '{{count}} match',
    matches_other: '{{count}} matches',
  },
};

beforeEach(async () => {
  await i18n.changeLanguage('zh-CN');
});

afterEach(async () => {
  await i18n.changeLanguage('zh-CN');
  i18n.removeResourceBundle('en', 'translation');
  i18n.removeResourceBundle('de', 'translation');
  vi.restoreAllMocks();
});

describe('应用翻译入口', () => {
  it('交付的英文资源按数量选择复数，并使用英文日期格式', async () => {
    await i18n.changeLanguage('en-US');
    expect(t('scan.matches', { count: 1 })).toBe('1 match');
    expect(t('scan.matches', { count: 2 })).toBe('2 matches');
    expect(t('scan.linkedSummary', { id: 7, count: 1 })).toBe('Scan #7 is linked to 1 archive');
    expect(t('scan.linkedSummary', { id: 7, count: 2 })).toBe('Scan #7 is linked to 2 archives');
    expect(d(new Date(2026, 0, 2), 'date')).toBe('01/02/2026');
  });

  it('嵌套消息插入原始文本，Vue 渲染负责 HTML 转义', () => {
    expect(t('scan.ocrValue', { text: '<OEM & mirror>' })).toBe('OCR 结果：<OEM & mirror>');
  });

  it('语言切换同时更新 Vue 翻译、普通模块的 computed 和数字展示', async () => {
    const app = createApp({});
    app.use(I18NextVue, { i18next: i18n });
    const componentText = app.runWithContext(() => {
      const { t } = useAppI18n();
      return computed(() => t('settings.save.failed.title'));
    });
    const moduleText = computed(() => t('settings.save.failed.title'));
    const percentage = computed(() => n(0.5, 'percent'));
    const number = computed(() => n(1234.5));
    expect(componentText.value).toBe('设置未保存');
    expect(moduleText.value).toBe('设置未保存');
    expect(percentage.value).toBe('50%');

    i18n.addResourceBundle('en', 'translation', englishFixture);
    await i18n.changeLanguage('en');
    await nextTick();
    expect(locale.value).toBe('en');
    expect(componentText.value).toBe('Settings not saved');
    expect(moduleText.value).toBe('Settings not saved');
    expect(percentage.value).toBe('50%');
    expect(t('scan.matches', { count: 1 })).toBe('1 match');
    expect(t('scan.matches', { count: 2 })).toBe('2 matches');
    expect(number.value).toBe('1,234.5');
    i18n.addResourceBundle('de', 'translation', englishFixture);
    await i18n.changeLanguage('de');
    await nextTick();
    expect(number.value).toBe('1.234,5');
    expect(percentage.value).toBe('50\u00a0%');
  });

  it('缺译显示原始 key 并记录当前语言，不回退中文', async () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => {});
    i18n.addResourceBundle('en', 'translation', englishFixture);
    await i18n.changeLanguage('en');
    expect(t('common.retrySave')).toBe('common.retrySave');
    expect(warning).toHaveBeenCalledWith('Missing translation', {
      locale: 'en',
      key: 'common.retrySave',
    });
  });

  it('组件插值保留可交互的链接，不把命名槽显示成占位符', async () => {
    const app = createSSRApp({
      setup() {
        const { t } = useAppI18n();
        return () =>
          h('p', [
            h(
              TranslationComponent,
              {
                translation: t('settings.update.mirror.service'),
              },
              {
                service: () => h('a', { href: 'https://mirrorchyan.com/' }, 'MirrorChyan'),
              },
            ),
          ]);
      },
    });
    app.use(I18NextVue, { i18next: i18n });
    const html = await renderToString(app);
    expect(html).toContain('<a href="https://mirrorchyan.com/">MirrorChyan</a>');
    expect(html).toContain(' 是独立的第三方加速下载服务，需要付费使用。');
    expect(html).not.toContain('{service}');
  });

  it('中文数量使用 other 类别，日期保留系统时区与秒精度', () => {
    expect(t('scan.matches', { count: 2 })).toBe('2 个匹配');
    const date = new Date(2026, 0, 2, 15, 4, 5);
    expect(d(date, 'dateTimeSeconds')).toBe('2026/01/02 15:04:05');
    expect(date.getHours()).toBe(15);
  });
});
