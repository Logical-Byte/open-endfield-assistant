import { defaultTheme } from '@/utils/theme/defaultTheme';
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

interface FontOption {
  label: string;
  value: string;
  family: string;
  source:
    | { type: 'use-chinese' }
    | { type: 'keyword' }
    | { type: 'local' }
    | { type: 'link'; links: { id: string; rel: string; href: string }[] };
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

/** Tailwind 颜色 id 对应的中文显示名。 */
const COLOR_ZH_NAMES: Record<string, string> = {
  red: '胭脂红',
  orange: '丹霞橙',
  amber: '琥珀黄',
  yellow: '鎏金黄',
  lime: '青柠绿',
  green: '帽子绿',
  emerald: '松石绿',
  teal: '孔雀青',
  cyan: '琉璃青',
  sky: '星河蓝',
  blue: '霜月蓝',
  indigo: '星夜靛',
  violet: '鸢尾紫',
  purple: '霞光紫',
  fuchsia: '丁香紫',
  pink: '少女粉',
  rose: '玫瑰红',
  slate: '烟青灰',
  gray: '钛金灰',
  zinc: '铅华灰',
  neutral: '珍珠灰',
  stone: '暖石灰',
  taupe: '亚麻褐',
  mauve: '淡霞紫',
  mist: '茶褐绿',
  olive: '橄榄绿',
  black: '玄墨黑',
  white: '象牙白',
};

/** 返回颜色的中文显示名，未收录时回退到英文 id。 */
function colorZhName(colorName: string): string {
  return COLOR_ZH_NAMES[colorName] ?? colorName;
}

function toColorEntry(colorName: string): ColorEntry {
  return {
    id: colorName,
    lightLabel: colorZhName(colorName),
    darkLabel: colorZhName(colorName),
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
const primaryColors: ColorEntry[] = [
  {
    id: 'grayscale',
    lightLabel: colorZhName('black'),
    darkLabel: colorZhName('white'),
    chipStyle: {
      '--color-light': 'black',
      '--color-dark': 'white',
    },
  },
  ...Object.keys(colors)
    .filter((colorName) => !colorsToOmit.includes(colorName))
    .filter((colorName) => !neutralColorNames.includes(colorName))
    .map(toColorEntry),
];
const secondaryColors = [...primaryColors];
const neutralColors = neutralColorNames.map(toColorEntry);

const radiuses: RadiusPreset[] = [
  { value: 0, label: '无' },
  { value: 0.125, label: '小' },
  { value: 0.25, label: '中' },
  { value: 0.375, label: '较大' },
  { value: 0.5, label: '大' },
];

const cornerShapePresets: CornerShapePreset[] = [
  {
    label: '内凹',
    value: '-infinity',
    cssValue: 'notch',
    coefficient: 0.46325137517610426,
  },
  {
    label: '斜切',
    value: '0',
    cssValue: 'bevel',
    coefficient: 0.6551363775620336,
  },
  {
    label: '标准',
    value: '1',
    cssValue: 'round',
    coefficient: 1,
  },
  {
    label: '柔和',
    value: 'log2(3)',
    cssValue: 'superellipse(log(3, 2))',
    coefficient: 1.3561800271129498,
  },
  {
    label: '平滑',
    value: '2',
    cssValue: 'squircle',
    coefficient: 1.7150089225301701,
  },
];

const supportsCornerShape = CSS.supports('corner-shape: squircle');

const englishFontOptions: FontOption[] = [
  { label: '（使用中文字体）', value: 'use-chinese', family: '', source: { type: 'use-chinese' } },
  {
    label: 'Public Sans',
    value: 'public-sans',
    family: 'Public Sans',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-public-sans',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Public+Sans:ital,wght@0,100..900;1,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'DM Sans',
    value: 'dm-sans',
    family: 'DM Sans',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-dm-sans',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=DM+Sans:ital,wght@0,100..900;1,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Geist',
    value: 'geist',
    family: 'Geist',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-geist',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Geist:ital,wght@0,100..900;1,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Inter',
    value: 'inter',
    family: 'Inter',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-inter',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Poppins',
    value: 'poppins',
    family: 'Poppins',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-poppins',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Poppins:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Outfit',
    value: 'outfit',
    family: 'Outfit',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-outfit',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Outfit:wght@100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Raleway',
    value: 'raleway',
    family: 'Raleway',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-raleway',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Raleway:ital,wght@0,100..900;1,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Google Sans Flex',
    value: 'google-sans-flex',
    family: 'Google Sans Flex',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-google-sans-flex',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Google+Sans+Flex:opsz,slnt,wdth,wght,GRAD,ROND@6..144,-10..0,25..151,1..1000,0..100,0..100&display=swap',
        },
      ],
    },
  },
  {
    label: 'Space Grotesk',
    value: 'space-grotesk',
    family: 'Space Grotesk',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-space-grotesk',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@300..700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Open Sans',
    value: 'open-sans',
    family: 'Open Sans',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-open-sans',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Open+Sans:ital,wdth,wght@0,75..100,300..800;1,75..100,300..800&display=swap',
        },
      ],
    },
  },
  {
    label: 'CMU Serif',
    value: 'cmu-serif',
    family: 'CMU Serif',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-cmu-serif',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/computer-modern@0.1.3/index.min.css',
        },
      ],
    },
  },
  {
    label: 'CMU Bright',
    value: 'cmu-bright',
    family: 'CMU Bright',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-cmu-bright',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/computer-modern@0.1.3/index.min.css',
        },
      ],
    },
  },
  {
    label: 'CMU Sans Serif',
    value: 'cmu-sans-serif',
    family: 'CMU Sans Serif',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-cmu-sans-serif',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/computer-modern@0.1.3/index.min.css',
        },
      ],
    },
  },
  {
    label: 'Latin Modern Roman',
    value: 'latin-modern-roman',
    family: 'TypoPRO Latin Modern Roman',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-latin-modern-roman',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/@typopro/web-latin-modern@3.7.5/TypoPRO-LatinModern.min.css',
        },
      ],
    },
  },
  {
    label: 'Latin Modern Sans',
    value: 'latin-modern-sans',
    family: 'TypoPRO Latin Modern Sans',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-latin-modern-sans',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/@typopro/web-latin-modern@3.7.5/TypoPRO-LatinModern.min.css',
        },
      ],
    },
  },
  { label: '（系统默认）', value: 'system-ui', family: 'system-ui', source: { type: 'keyword' } },
  {
    label: '（浏览器默认）',
    value: 'sans-serif',
    family: 'sans-serif',
    source: { type: 'keyword' },
  },
];

const chineseFontOptions: FontOption[] = [
  {
    label: '鸿蒙黑体',
    value: 'harmonyos-sans-sc',
    family: 'HarmonyOS Sans SC',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-harmonyos-sans-sc',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/gh/BiologyHazard/harmonyos-sans-sc@1.0.0/result.css',
        },
      ],
    },
  },
  {
    label: '阿里巴巴普惠体',
    value: 'alibaba-puhuiti',
    family: 'Alibaba PuHuiTi 3.0',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-alibaba-puhuiti',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/gh/BiologyHazard/alibaba-puhuiti-3.0@1.0.0/result.css',
        },
      ],
    },
  },
  {
    label: '思源黑体',
    value: 'noto-sans-sc',
    family: 'Noto Sans SC',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-noto-sans-sc',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Noto+Sans+SC:wght@100..900&display=swap',
        },
      ],
    },
  },
  {
    label: '思源宋体',
    value: 'noto-serif-sc',
    family: 'Noto Serif SC',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-noto-serif-sc',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Noto+Serif+SC:wght@200..900&display=swap',
        },
      ],
    },
  },
  {
    label: '霞鹜文楷',
    value: 'lxgw-wenkai',
    family: 'LXGW WenKai',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-lxgw-wenkai',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/lxgw-wenkai-webfont@1.7.0/style.min.css',
          // href: 'https://cn-font.claude-code-best.win/packages/lxgwwenkai/dist/LXGWWenKai-Regular/result.css',
          // href: 'https://cn-font.claude-code-best.win/packages/lxgwwenkai/dist/LXGWWenKai-Light/result.css',
          // href: 'https://cn-font.claude-code-best.win/packages/lxgwwenkai/dist/LXGWWenKai-Bold/result.css',
        },
      ],
    },
  },
  {
    label: '霞鹜文楷屏幕阅读版',
    value: 'lxgw-wenkai-screen',
    family: 'LXGW WenKai Screen',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-lxgw-wenkai-screen',
          rel: 'stylesheet',
          href: 'https://cdn.jsdelivr.net/npm/lxgw-wenkai-screen-webfont@1.7.0/style.min.css',
          // href: 'https://cn-font.claude-code-best.win/packages/lywkpmydb/dist/LXGWWenKaiScreen/result.css',
        },
      ],
    },
  },
  { label: '（系统默认）', value: 'system-ui', family: 'system-ui', source: { type: 'keyword' } },
  {
    label: '（浏览器默认）',
    value: 'sans-serif',
    family: 'sans-serif',
    source: { type: 'keyword' },
  },
];

const monospaceFontOptions: FontOption[] = [
  {
    label: 'JetBrains Mono',
    value: 'jetbrains-mono',
    family: 'JetBrains Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-jetbrains-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:ital,wght@0,100..800;1,100..800&display=swap',
        },
      ],
    },
  },
  {
    label: 'Google Sans Code',
    value: 'google-sans-code',
    family: 'Google Sans Code',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-google-sans-code',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Google+Sans+Code:ital,wght,MONO@0,300..800,0..1;1,300..800,0..1&display=swap',
        },
      ],
    },
  },
  {
    label: 'Fira Code',
    value: 'fira-code',
    family: 'Fira Code',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-fira-code',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Fira+Code:wght@300..700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Fira Mono',
    value: 'fira-mono',
    family: 'Fira Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-fira-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Fira+Mono:wght@400;500;700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Source Code Pro',
    value: 'source-code-pro',
    family: 'Source Code Pro',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-source-code-pro',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Source+Code+Pro:ital,wght@0,200..900;1,200..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Noto Sans Mono',
    value: 'noto-sans-mono',
    family: 'Noto Sans Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-noto-sans-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Noto+Sans+Mono:wdth,wght@62.5..100,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Space Mono',
    value: 'space-mono',
    family: 'Space Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-space-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Space+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Cascadia Code',
    value: 'cascadia-code',
    family: 'Cascadia Code',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-cascadia-code',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Cascadia+Code:ital,wght@0,200..700;1,200..700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Cascadia Mono',
    value: 'cascadia-mono',
    family: 'Cascadia Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-cascadia-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Cascadia+Mono:ital,wght@0,200..700;1,200..700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Geist Mono',
    value: 'geist-mono',
    family: 'Geist Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-geist-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Geist+Mono:ital,wght@0,100..900;1,100..900&display=swap',
        },
      ],
    },
  },
  {
    label: 'Ubuntu Mono',
    value: 'ubuntu-mono',
    family: 'Ubuntu Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-ubuntu-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Ubuntu+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap',
        },
      ],
    },
  },
  {
    label: 'Ubuntu Sans Mono',
    value: 'ubuntu-sans-mono',
    family: 'Ubuntu Sans Mono',
    source: {
      type: 'link',
      links: [
        {
          id: 'font-ubuntu-sans-mono',
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Ubuntu+Sans+Mono:ital,wght@0,400..700;1,400..700&display=swap',
        },
      ],
    },
  },
  { label: '（系统默认）', value: 'system-ui', family: 'system-ui', source: { type: 'keyword' } },
  {
    label: '（浏览器默认）',
    value: 'monospace',
    family: 'monospace',
    source: { type: 'keyword' },
  },
];

const colorModes = computed<{ label: string; value: 'light' | 'dark' | 'auto'; icon: string }[]>(
  () => [
    { label: '浅色模式', value: 'light', icon: appConfig.ui.icons.light },
    { label: '深色模式', value: 'dark', icon: appConfig.ui.icons.dark },
    { label: '跟随系统', value: 'auto', icon: appConfig.ui.icons.system },
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

/** 圆角半径（单位：rem） */
const radius = useStorage<number>('oea:theme.radius', 0.25);

/** 圆角形状预设值（对应 cornerShapePresets 中的 value） */
const cornerShape = useStorage<string>(
  'oea:theme.cornerShape',
  supportsCornerShape ? 'log2(3)' : '1',
);

/** 当前选中的圆角形状 */
const selectedCornerShape = computed<CornerShapePreset | undefined>(() =>
  cornerShapePresets.find((p) => p.value === cornerShape.value),
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
const englishFont = useStorage<string>('oea:theme.englishFont', 'use-chinese');
/** 选中的中文字体 ID */
const chineseFont = useStorage<string>('oea:theme.chineseFont', 'harmonyos-sans-sc');
/** 选中的等宽字体 ID */
const monospaceFont = useStorage<string>('oea:theme.monospaceFont', 'jetbrains-mono');

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

function loadEnglishFontCss() {
  loadFontCss(englishFontOptions);
}

function loadChineseFontCss() {
  loadFontCss(chineseFontOptions);
}

function loadMonospaceFontCss() {
  loadFontCss(monospaceFontOptions);
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
  radius.value = 0.25;
  cornerShape.value = supportsCornerShape ? 'log2(3)' : '1';
  englishFont.value = 'use-chinese';
  chineseFont.value = 'harmonyos-sans-sc';
  monospaceFont.value = 'jetbrains-mono';
}

export function useTheme() {
  return {
    primaryColors,
    secondaryColors,
    neutralColors,
    radiuses,
    cornerShapePresets,
    supportsCornerShape,
    englishFontOptions,
    chineseFontOptions,
    monospaceFontOptions,
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
    loadEnglishFontCss,
    loadChineseFontCss,
    loadMonospaceFontCss,
    getPreviewFontFamily,
    resetTheme,
  };
}
