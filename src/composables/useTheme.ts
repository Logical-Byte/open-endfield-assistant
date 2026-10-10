import { t, type MessageKey } from '@/shared/i18n';
import { defaultTheme } from '@/features/appearance/defaultTheme';
import {
  englishFontOptions,
  chineseFontOptions,
  monospaceFontOptions,
  type FontOption,
} from '@/features/appearance/fonts';
import type { ResolvableLink, ResolvableStyle } from '@unhead/vue';
import { useStorage } from '@vueuse/core';
import colors from 'tailwindcss/colors';
import { computed, ref, watch, type CSSProperties } from 'vue';

const appConfig = useAppConfig();

interface ColorEntry {
  id: string;
  lightLabel: string;
  darkLabel: string;
  chipStyle: CSSProperties;
}

interface RadiusPreset {
  value: number;
  label: string;
}

interface CornerShapePreset {
  label: string;
  value: string;
  cssValue: string;
  coefficient: number;
}

/**
 * 从 `tailwindcss/colors` 的 JavaScript 对象中获取指定颜色的具体色值。
 * 用于在 CSS var() fallback 中提供硬编码颜色值，避免完全依赖 Tailwind CSS 变量。
 */
function getTailwindColor(colorName: string, shade: number): string | null {
  const palette = (colors as Record<string, Record<string, string> | string>)[colorName];
  if (typeof palette === 'object') {
    return palette[shade] ?? null;
  } else if (typeof palette === 'string') {
    return palette;
  } else {
    return null;
  }
}

/**
 * 获取颜色的 CSS 属性值，格式为 `var(--color-<name>-<shade>, <fallback>)`。
 *
 * 采用 “CSS 变量优先 + 具体值回退” 策略：
 * - 优先使用 Tailwind CSS 变量（尊重 `@theme` 中的颜色覆盖）。
 * - 若变量被 Tailwind 的 tree-shaking 移除，则 fallback 到从 `tailwindcss/colors` 提取的具体色值。
 * - `neutral` 颜色会映射为 `old-neutral`，以适配 Nuxt UI 的主题重写。
 */
function getColorCSSProperty(colorName: string, shade: number): string {
  // Nuxt UI 主题会重写 --color-neutral-* 为 --color-old-neutral-*。
  const colorVarName = colorName === 'neutral' ? 'old-neutral' : colorName;
  const fallback = getTailwindColor(colorName, shade);
  if (fallback !== null) {
    return `var(--color-${colorVarName}-${shade}, ${fallback})`;
  } else {
    return `var(--color-${colorVarName}-${shade})`;
  }
}

/** 颜色选项只重算标签，颜色 ID 与 CSS 保持不变。 */
const colorLabels: Record<string, MessageKey> = {
  red: 'theme.color.red',
  orange: 'theme.color.orange',
  amber: 'theme.color.amber',
  yellow: 'theme.color.yellow',
  lime: 'theme.color.lime',
  green: 'theme.color.green',
  emerald: 'theme.color.emerald',
  teal: 'theme.color.teal',
  cyan: 'theme.color.cyan',
  sky: 'theme.color.sky',
  blue: 'theme.color.blue',
  indigo: 'theme.color.indigo',
  violet: 'theme.color.violet',
  purple: 'theme.color.purple',
  fuchsia: 'theme.color.fuchsia',
  pink: 'theme.color.pink',
  rose: 'theme.color.rose',
  slate: 'theme.color.slate',
  gray: 'theme.color.gray',
  zinc: 'theme.color.zinc',
  neutral: 'theme.color.neutral',
  stone: 'theme.color.stone',
  taupe: 'theme.color.taupe',
  mauve: 'theme.color.mauve',
  mist: 'theme.color.mist',
  olive: 'theme.color.olive',
  black: 'theme.color.black',
  white: 'theme.color.white',
};
function colorLabel(colorName: string): string {
  const key = colorLabels[colorName];
  return key ? t(key) : colorName;
}

function toColorEntry(colorName: string): ColorEntry {
  return {
    id: colorName,
    lightLabel: colorLabel(colorName),
    darkLabel: colorLabel(colorName),
    chipStyle: {
      // 优先使用 CSS 变量（尊重 @theme 覆盖），被 tree-shake 时 fallback 到具体值
      '--color-light': getColorCSSProperty(colorName, 500),
      '--color-dark': getColorCSSProperty(colorName, 400),
    },
  };
}

const colorsToOmit = ['inherit', 'current', 'transparent', 'black', 'white'];
const neutralColorNames = [
  'slate',
  'gray',
  'zinc',
  'neutral',
  'stone',
  'taupe',
  'mauve',
  'mist',
  'olive',
];
const primaryColors = computed<ColorEntry[]>(() => [
  {
    id: 'grayscale',
    lightLabel: colorLabel('black'),
    darkLabel: colorLabel('white'),
    chipStyle: {
      '--color-light': 'black',
      '--color-dark': 'white',
    },
  },
  ...Object.keys(colors)
    .filter((colorName) => !colorsToOmit.includes(colorName))
    .filter((colorName) => !neutralColorNames.includes(colorName))
    .map(toColorEntry),
]);
const secondaryColors = computed(() => [...primaryColors.value]);
const neutralColors = computed(() => neutralColorNames.map(toColorEntry));

const radiuses = computed<RadiusPreset[]>(() => [
  { value: 0, label: t('theme.radius.none') },
  { value: 0.125, label: t('theme.radius.small') },
  { value: 0.25, label: t('theme.radius.medium') },
  { value: 0.375, label: t('theme.radius.larger') },
  { value: 0.5, label: t('theme.radius.large') },
]);

const cornerShapePresets = computed<CornerShapePreset[]>(() => [
  {
    label: t('theme.shape.notch'),
    value: '-infinity',
    cssValue: 'notch',
    coefficient: 0.46325137517610426,
  },
  {
    label: t('theme.shape.bevel'),
    value: '0',
    cssValue: 'bevel',
    coefficient: 0.6551363775620336,
  },
  {
    label: t('theme.shape.round'),
    value: '1',
    cssValue: 'round',
    coefficient: 1,
  },
  {
    label: t('theme.shape.soft'),
    value: 'log2(3)',
    cssValue: 'superellipse(log(3, 2))',
    coefficient: 1.3561800271129498,
  },
  {
    label: t('theme.shape.smooth'),
    value: '2',
    cssValue: 'squircle',
    coefficient: 1.7150089225301701,
  },
]);

const supportsCornerShape = CSS.supports('corner-shape: squircle');

const colorModes = computed<{ label: string; value: 'light' | 'dark' | 'auto'; icon: string }[]>(
  () => [
    { label: t('theme.mode.light'), value: 'light', icon: appConfig.ui.icons.light },
    { label: t('theme.mode.dark'), value: 'dark', icon: appConfig.ui.icons.dark },
    { label: t('theme.mode.auto'), value: 'auto', icon: appConfig.ui.icons.system },
  ],
);

// 主题设置持久化：所有可自定义的选项都通过 useStorage 写入 localStorage，重启应用后自动恢复。
const primary = useStorage<string>('oea:theme.primary', appConfig.ui.colors.primary);
const secondary = useStorage<string>('oea:theme.secondary', appConfig.ui.colors.secondary);
const neutral = useStorage<string>('oea:theme.neutral', appConfig.ui.colors.neutral);

// 颜色变更时同步到 appConfig（Nuxt UI 依赖 appConfig.ui.colors 应用主题色）。
// immediate: true 使启动时也执行一次，用持久化的值初始化 appConfig。
// grayscale 不写 appConfig，由注入的 CSS 变量覆盖 --ui-primary/--ui-secondary。
watch(
  [primary, secondary, neutral],
  ([newPrimary, newSecondary, newNeutral]) => {
    if (newPrimary !== 'grayscale') {
      appConfig.ui.colors.primary = newPrimary;
    }
    if (newSecondary !== 'grayscale') {
      appConfig.ui.colors.secondary = newSecondary;
    }
    appConfig.ui.colors.neutral = newNeutral;
  },
  { immediate: true },
);

/** 本地主题偏好的初始化与重置默认值。 */
const themeDefaults = {
  radius: 0.25,
  cornerShape: supportsCornerShape ? 'log2(3)' : '1',
  englishFont: 'use-chinese',
  chineseFont: 'harmonyos-sans-sc',
  monospaceFont: 'jetbrains-mono',
};

/** 圆角半径（单位：rem） */
const radius = useStorage<number>('oea:theme.radius', themeDefaults.radius);

/** 圆角形状预设值（对应 cornerShapePresets 中的 value） */
const cornerShape = useStorage<string>('oea:theme.cornerShape', themeDefaults.cornerShape);

/** 当前选中的圆角形状 */
const selectedCornerShape = computed<CornerShapePreset | undefined>(() =>
  cornerShapePresets.value.find((p) => p.value === cornerShape.value),
);

/** 当前选中圆角形状的补偿系数 */
const cornerShapeCoefficient = computed<number>(() => {
  if (!supportsCornerShape) {
    return 1;
  } else {
    return selectedCornerShape.value?.coefficient ?? 1;
  }
});

/** 选中的英文字体 ID */
const englishFont = useStorage<string>('oea:theme.englishFont', themeDefaults.englishFont);
/** 选中的中文字体 ID */
const chineseFont = useStorage<string>('oea:theme.chineseFont', themeDefaults.chineseFont);
/** 选中的等宽字体 ID */
const monospaceFont = useStorage<string>('oea:theme.monospaceFont', themeDefaults.monospaceFont);

/** 选中的英文字体配置 */
const englishFontOption = computed<FontOption | undefined>(() =>
  englishFontOptions.find((font) => font.value === englishFont.value),
);
/** 选中的中文字体配置 */
const chineseFontOption = computed<FontOption | undefined>(() =>
  chineseFontOptions.find((font) => font.value === chineseFont.value),
);
/** 选中的等宽字体配置 */
const monospaceFontOption = computed<FontOption | undefined>(() =>
  monospaceFontOptions.find((font) => font.value === monospaceFont.value),
);

/**
 * 将字体转换为 CSS 字体家族名称字符串
 * 如果字体为关键字类型，则直接返回字体名称，否则加上单引号
 * @example
 * toCssFontFamily({ family: 'Public Sans', source: { type: 'link', links: [...] } }) // => "'Public Sans'"
 * toCssFontFamily({ family: 'system-ui', source: { type: 'keyword' } }) // => "system-ui"
 */
function toCssFontFamily(font: FontOption): string {
  return font.source.type === 'keyword' ? font.family : `'${font.family}'`;
}

const style = computed<ResolvableStyle[]>(() => {
  const style: ResolvableStyle[] = [];

  // 主题色为 grayscale 时，设置 --ui-primary 和 --ui-secondary 变量为黑白色
  // tagPriority 必须大于 nuxt-ui-colors 的实际权重（"critical" = style 基础 60 + (-8) = 52），
  // 否则 unhead 首次渲染时我们的覆盖会排在它前面，同一 @layer theme 内被它覆盖（刷新后失效）。
  if (primary.value === 'grayscale') {
    style.push({
      innerHTML: `@layer theme { :root { --ui-primary: black; } .dark { --ui-primary: white; } }`,
      id: 'nuxt-ui-primary-grayscale',
      tagPriority: 60,
    });
  }
  if (secondary.value === 'grayscale') {
    style.push({
      innerHTML: `@layer theme { :root { --ui-secondary: black; } .dark { --ui-secondary: white; } }`,
      id: 'nuxt-ui-secondary-grayscale',
      tagPriority: 60,
    });
  }

  // 圆角大小
  // 浏览器支持 corner-shape: squircle 时，根据预设系数补偿圆角半径
  const effectiveRadius = supportsCornerShape
    ? radius.value * cornerShapeCoefficient.value
    : radius.value;
  style.push({
    innerHTML: `@layer theme { :root { --ui-radius: ${effectiveRadius}rem; --ui-radius-initial: ${radius.value}rem; --corner-shape-coefficient: ${cornerShapeCoefficient.value}; } }`,
    id: 'nuxt-ui-radius',
    tagPriority: -2,
  });

  // 圆角形状（superellipse 参数）
  // 与 squircle-corner.css 中的 var(--corner-shape) 配合使用
  if (supportsCornerShape && selectedCornerShape.value) {
    style.push({
      innerHTML: `@layer theme { :root { --corner-shape: ${selectedCornerShape.value.cssValue}; } }`,
      id: 'nuxt-ui-corner-shape',
      tagPriority: -2,
    });
  }

  // 字体
  // 将字体选项转换为 CSS 字体家族名称字符串
  const englishFontCss = englishFontOption.value
    ? toCssFontFamily(englishFontOption.value)
    : `'${englishFont.value}'`;
  const chineseFontCss = chineseFontOption.value
    ? toCssFontFamily(chineseFontOption.value)
    : `'${chineseFont.value}'`;
  const monospaceFontCss = monospaceFontOption.value
    ? toCssFontFamily(monospaceFontOption.value)
    : `'${monospaceFont.value}'`;

  // 根据选中的字体生成 CSS 变量
  const fontSansInnerHtml =
    englishFontOption.value?.source.type === 'use-chinese'
      ? `@layer theme { :root { --font-sans: ${chineseFontCss}, sans-serif; } }` // 如果英文字体为 “使用中文字体”，则只使用中文字体和 sans-serif
      : `@layer theme { :root { --font-sans: ${englishFontCss}, ${chineseFontCss}, sans-serif; } }`;
  const fontMonoInnerHtml = `@layer theme { :root { --font-mono: ${monospaceFontCss}, ${chineseFontCss}, monospace; } }`;

  // 将 CSS 变量添加到 style 中
  style.push({
    innerHTML: fontSansInnerHtml,
    id: 'nuxt-ui-sans-font',
    tagPriority: -2,
  });
  style.push({
    innerHTML: fontMonoInnerHtml,
    id: 'nuxt-ui-mono-font',
    tagPriority: -2,
  });

  return style;
});

const extraFontLinks = ref<ResolvableLink[]>([]);

const link = computed<ResolvableLink[]>(() => {
  // 当前已选字体（始终需要）
  const selectedFonts: FontOption[] = [];

  if (englishFontOption.value) selectedFonts.push(englishFontOption.value);
  if (chineseFontOption.value) selectedFonts.push(chineseFontOption.value);
  if (monospaceFontOption.value) selectedFonts.push(monospaceFontOption.value);

  const selectedLinks: ResolvableLink[] = selectedFonts.flatMap((font) =>
    font.source.type === 'link' ? font.source.links : [],
  );

  // 合并按需加载的预览字体，按 id 去重
  const seenIds = new Set(selectedLinks.map((l) => l.id));
  const allLinks: ResolvableLink[] = [...selectedLinks];
  for (const link of extraFontLinks.value) {
    if (!seenIds.has(link.id)) {
      seenIds.add(link.id);
      allLinks.push(link);
    }
  }
  return allLinks;
});

/** 按需加载字体 CSS（通过 extraFontLinks 响应式合并到 link），用于字体选择框预览 */
function loadFontCss(fontOptions: FontOption[]): void {
  const existingIds = new Set(extraFontLinks.value.map((l) => l.id));
  const newLinks: ResolvableLink[] = [];
  for (const font of fontOptions) {
    if (font.source.type !== 'link') {
      continue;
    }
    for (const linkDef of font.source.links) {
      if (!existingIds.has(linkDef.id)) {
        existingIds.add(linkDef.id);
        newLinks.push(linkDef);
      }
    }
  }
  if (newLinks.length > 0) {
    extraFontLinks.value = [...extraFontLinks.value, ...newLinks];
  }
}

function getPreviewFontFamily(
  type: 'english' | 'chinese' | 'monospace',
  item: FontOption,
): string | undefined {
  switch (type) {
    case 'english':
      if (item.source.type === 'use-chinese') {
        return undefined;
      } else {
        return `${toCssFontFamily(item)}, sans-serif`;
      }
    case 'chinese':
      return `${toCssFontFamily(item)}, sans-serif`;
    case 'monospace':
      return `${toCssFontFamily(item)}, monospace`;
  }
}

function resetTheme() {
  primary.value = defaultTheme.ui.colors.primary;
  secondary.value = defaultTheme.ui.colors.secondary;
  neutral.value = defaultTheme.ui.colors.neutral;
  radius.value = themeDefaults.radius;
  cornerShape.value = themeDefaults.cornerShape;
  englishFont.value = themeDefaults.englishFont;
  chineseFont.value = themeDefaults.chineseFont;
  monospaceFont.value = themeDefaults.monospaceFont;
}

const fontLabelKeys: Record<string, MessageKey> = {
  'use-chinese': 'theme.font.useChinese',
  'system-ui': 'theme.font.system',
  'sans-serif': 'theme.font.browser',
  monospace: 'theme.font.browser',
};
function localizedFonts(options: FontOption[]): FontOption[] {
  return options.map((option) => ({
    ...option,
    label: fontLabelKeys[option.value] ? t(fontLabelKeys[option.value]) : option.label,
  }));
}
const localizedEnglishFonts = computed(() => localizedFonts(englishFontOptions));
const localizedChineseFonts = computed(() => localizedFonts(chineseFontOptions));
const localizedMonospaceFonts = computed(() => localizedFonts(monospaceFontOptions));

export function useTheme() {
  return {
    primaryColors,
    secondaryColors,
    neutralColors,
    radiuses,
    cornerShapePresets,
    supportsCornerShape,
    englishFontOptions: localizedEnglishFonts,
    chineseFontOptions: localizedChineseFonts,
    monospaceFontOptions: localizedMonospaceFonts,
    colorModes,
    primary,
    secondary,
    neutral,
    radius,
    cornerShape,
    cornerShapeCoefficient,
    englishFont,
    chineseFont,
    monospaceFont,
    style,
    link,
    loadFontCss,
    getPreviewFontFamily,
    resetTheme,
  };
}
