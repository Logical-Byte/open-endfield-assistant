export interface FontOption {
  label: string;
  value: string;
  family: string;
  source:
    | { type: 'use-chinese' }
    | { type: 'keyword' }
    | { type: 'local' }
    | { type: 'link'; links: { id: string; rel: string; href: string }[] };
}

export const englishFontOptions: FontOption[] = [
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

export const chineseFontOptions: FontOption[] = [
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

export const monospaceFontOptions: FontOption[] = [
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
