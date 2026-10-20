import { ref, shallowRef } from 'vue';

/** 页面与应用侧栏共享分类定义，标题在使用处翻译以响应语言切换。 */
export const settingsSections = [
  { id: 'language', icon: 'i-lucide-languages', titleKey: 'settings.language.section' },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', titleKey: 'settings.interface.section' },
  { id: 'sound', icon: 'i-lucide-headphones', titleKey: 'settings.sound.section' },
  { id: 'update', icon: 'i-lucide-download', titleKey: 'settings.update.section' },
  { id: 'developer', icon: 'i-lucide-code-2', titleKey: 'settings.developer.section' },
  { id: 'about', icon: 'i-lucide-info', titleKey: 'settings.about.title' },
] as const;

export const helpSections = [
  { id: 'getting-started', icon: 'i-lucide-rocket', titleKey: 'help.start.title' },
  { id: 'usage', icon: 'i-lucide-keyboard', titleKey: 'help.usage.title' },
  { id: 'faq', icon: 'i-lucide-circle-help', titleKey: 'help.faq.title' },
  { id: 'feedback', icon: 'i-lucide-message-circle', titleKey: 'help.feedback.title' },
  { id: 'credits', icon: 'i-lucide-heart', titleKey: 'help.credits.title' },
  { id: 'disclaimer', icon: 'i-lucide-file-text', titleKey: 'help.disclaimer.title' },
] as const;

export type HelpSectionId = (typeof helpSections)[number]['id'];
export const activeHelpSectionId = ref<HelpSectionId>('getting-started');
// 每次点击生成一个请求，即使重复点击当前分类，也能重新定位到分类开头。
export const helpSectionRequest = shallowRef<{ id: HelpSectionId }>();
