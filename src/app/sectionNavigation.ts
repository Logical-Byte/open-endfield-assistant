import { ref, shallowRef } from 'vue';

/** 页面与应用侧栏共享分类定义，标题在使用处翻译以响应语言切换。 */
export const settingsSections = [
  // 语言设置必须保持第一栏，与设置页的默认分类一致，作为侧栏设置入口的语言切换路径。
  { id: 'language', icon: 'i-lucide-languages', titleKey: 'settings.sections.language' },
  { id: 'interface', icon: 'i-lucide-layout-panel-left', titleKey: 'settings.sections.interface' },
  { id: 'sound', icon: 'i-lucide-headphones', titleKey: 'settings.sections.sound' },
  { id: 'update', icon: 'i-lucide-download', titleKey: 'settings.sections.update' },
  { id: 'developer', icon: 'i-lucide-code-2', titleKey: 'settings.sections.developer' },
  { id: 'about', icon: 'i-lucide-info', titleKey: 'settings.sections.about' },
] as const;

export const helpSections = [
  { id: 'getting-started', icon: 'i-lucide-rocket', titleKey: 'help.sections.gettingStarted' },
  { id: 'archive-scan', icon: 'i-lucide-scan-line', titleKey: 'help.sections.archiveScan' },
  { id: 'gamble-battle', icon: 'i-lucide-dices', titleKey: 'help.sections.arena' },
  { id: 'usage', icon: 'i-lucide-keyboard', titleKey: 'help.sections.shortcuts' },
  { id: 'faq', icon: 'i-lucide-circle-help', titleKey: 'help.sections.faq' },
  { id: 'feedback', icon: 'i-lucide-message-circle', titleKey: 'help.sections.feedback' },
  { id: 'credits', icon: 'i-lucide-heart', titleKey: 'help.sections.credits' },
  { id: 'disclaimer', icon: 'i-lucide-file-text', titleKey: 'help.sections.disclaimer' },
] as const;

export type HelpSectionId = (typeof helpSections)[number]['id'];
export const activeHelpSectionId = ref<HelpSectionId>('getting-started');
// 每次点击生成一个请求，即使重复点击当前分类，也能重新定位到分类开头。
export const helpSectionRequest = shallowRef<{ id: HelpSectionId }>();
