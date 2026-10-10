import 'i18next';
import type zhCn from './locales/zh-CN.json';

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation';
    resources: { translation: typeof zhCn };
    returnNull: false;
    strictKeyChecks: true;
    // 保留静态字符串 key，便于翻译工具提取和现有调用迁移。
    enableSelector: false;
  }
}
