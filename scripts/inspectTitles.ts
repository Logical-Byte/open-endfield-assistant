/**
 * 临时脚本：按分类列出 prts.json 的全部标题，并标记 OCR 纠错风险。
 *
 * 运行：pnpm exec jiti scripts/inspectTitles.ts
 *
 * 输出：
 * - 控制台：每个分类的标题数 + 全部有风险的标题（含原因）
 * - temp/titles-inspection.txt：完整标题列表（含分类、风险标记）
 */
import fs from 'node:fs';

interface PrtsPage {
  name: string;
  pageType: string;
  categoryIds: string[];
}
interface PrtsCategory {
  name: string;
  order: number;
}
interface PrtsFirstLv {
  categoryId: string;
  firstLvId: string;
  order: number;
}
interface AllItem {
  id: string;
  title: string;
  categoryId: string;
  firstLvId: string;
  order: number;
  type: string;
}
interface PrtsData {
  PrtsPage: Record<string, PrtsPage>;
  PrtsCategory: Record<string, PrtsCategory>;
  firstLv: Record<string, PrtsFirstLv>;
  allItems: Record<string, AllItem>;
}

const data = JSON.parse(fs.readFileSync('resources/data/prts.json', 'utf8')) as PrtsData;

// ---------- 工具 ----------

function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  const m = a.length;
  const n = b.length;
  if (m === 0) return n;
  if (n === 0) return m;
  let prev = new Array<number>(n + 1);
  let curr = new Array<number>(n + 1);
  for (let j = 0; j <= n; j++) prev[j] = j;
  for (let i = 1; i <= m; i++) {
    curr[0] = i;
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      curr[j] = Math.min(curr[j - 1] + 1, prev[j] + 1, prev[j - 1] + cost);
    }
    [prev, curr] = [curr, prev];
  }
  return prev[n];
}

/** 常见繁/异体字 → 简体（OCR 可能把简体识别成繁体/异体） */
const TRAD: Record<string, string> = {
  決: '决',
  聲: '声',
  響: '响',
  樂: '乐',
  與: '与',
  於: '于',
  後: '后',
  發: '发',
  愛: '爱',
  點: '点',
  體: '体',
  讀: '读',
  認: '认',
  識: '识',
  記: '记',
  錄: '录',
  遺: '遗',
  驚: '惊',
  戰: '战',
  團: '团',
  員: '员',
  隊: '队',
  對: '对',
  備: '备',
  將: '将',
  來: '来',
  門: '门',
  問: '问',
  間: '间',
  聞: '闻',
  聽: '听',
  風: '风',
  飛: '飞',
  機: '机',
  氣: '气',
  燈: '灯',
  當: '当',
  異: '异',
  義: '义',
  數: '数',
  條: '条',
  類: '类',
  頭: '头',
  題: '题',
  驗: '验',
  紙: '纸',
  網: '网',
  舊: '旧',
  華: '华',
  藝: '艺',
  處: '处',
  衛: '卫',
  術: '术',
  話: '话',
  語: '语',
  說: '说',
  誰: '谁',
  調: '调',
  論: '论',
  講: '讲',
  課: '课',
  讓: '让',
  議: '议',
  貝: '贝',
  買: '买',
  賣: '卖',
  質: '质',
  轉: '转',
  農: '农',
  過: '过',
  達: '达',
  適: '适',
  還: '还',
  選: '选',
  鄉: '乡',
  醫: '医',
  雲: '云',
  電: '电',
  頁: '页',
  順: '顺',
  須: '须',
  飲: '饮',
  館: '馆',
  齊: '齐',
  龍: '龙',
  麼: '么',
  錢: '钱',
  錯: '错',
  鎮: '镇',
  長: '长',
  閉: '闭',
  開: '开',
  陽: '阳',
  陳: '陈',
  隨: '随',
  雜: '杂',
  嚴: '严',
  廣: '广',
  萬: '万',
  傷: '伤',
  價: '价',
  傑: '杰',
  勝: '胜',
  勞: '劳',
  動: '动',
  區: '区',
  協: '协',
  單: '单',
  國: '国',
  圖: '图',
  圓: '圆',
  場: '场',
  報: '报',
  學: '学',
  實: '实',
  寧: '宁',
  審: '审',
  專: '专',
  導: '导',
  師: '师',
  帶: '带',
  張: '张',
  復: '复',
  徑: '径',
  從: '从',
  懷: '怀',
  態: '态',
  戲: '戏',
  擴: '扩',
  敵: '敌',
  斬: '斩',
  書: '书',
  會: '会',
  曆: '历',
  殺: '杀',
  東: '东',
  構: '构',
  標: '标',
  橋: '桥',
  檢: '检',
  樓: '楼',
  歐: '欧',
  歸: '归',
  毀: '毁',
  沒: '没',
  況: '况',
  滿: '满',
  無: '无',
  煙: '烟',
  爾: '尔',
  現: '现',
  環: '环',
  產: '产',
  畫: '画',
  監: '监',
  盤: '盘',
  盡: '尽',
  確: '确',
  種: '种',
  積: '积',
  穩: '稳',
  範: '范',
  紀: '纪',
  線: '线',
  總: '总',
  繼: '继',
  羅: '罗',
  聖: '圣',
  興: '兴',
  蓋: '盖',
  號: '号',
  裝: '装',
  見: '见',
  觀: '观',
  覺: '觉',
  計: '计',
  訓: '训',
  設: '设',
  許: '许',
  該: '该',
  誠: '诚',
  誤: '误',
  請: '请',
  諸: '诸',
  變: '变',
  證: '证',
  護: '护',
  贏: '赢',
  車: '车',
  軍: '军',
  較: '较',
  輕: '轻',
  輪: '轮',
  軟: '软',
  辭: '辞',
  運: '运',
  週: '周',
  進: '进',
  違: '违',
  遠: '远',
  銀: '银',
  鎖: '锁',
  鍋: '锅',
  鍵: '键',
  鏡: '镜',
  閃: '闪',
  陸: '陆',
  隱: '隐',
  雖: '虽',
  難: '难',
  願: '愿',
  顯: '显',
  馬: '马',
  鳥: '鸟',
  黨: '党',
  龐: '庞',
};

/** 全角标点（OCR 可能识别成半角） */
const FULLWIDTH_PUNCT = /[（）：；，、！？《》「」『』【】“”‘’—…·]/;

/** 超过该长度有截断风险（OCR ROI 约一行） */
const TRUNCATE_THRESHOLD = 14;

// ---------- 风险检测 ----------

function detectRisks(title: string, siblings: string[]): string[] {
  const risks: string[] = [];
  const len = [...title].length;

  if (title.includes('<@') || title.includes('■')) {
    risks.push('富文本/打码');
  }
  if (len >= TRUNCATE_THRESHOLD) {
    risks.push(`长度${len}（可能截断）`);
  }
  if (FULLWIDTH_PUNCT.test(title)) {
    risks.push('全角标点');
  }
  const tradChars = [...title].filter((c) => TRAD[c]);
  if (tradChars.length > 0) {
    risks.push(`繁/异体字: ${tradChars.join('')}`);
  }
  if (/[A-Za-z0-9]/.test(title)) {
    risks.push('含英文/数字');
  }
  // 同分类内易混淆标题（编辑距离很小）
  const similar = siblings.filter(
    (s) => s !== title && levenshtein(title, s) <= Math.max(1, Math.floor(len * 0.15)),
  );
  if (similar.length > 0) {
    risks.push(`易混淆: ${similar.slice(0, 3).join('、')}`);
  }
  return risks;
}

// ---------- 主逻辑 ----------

function orderedItemsForCategory(categoryId: string): AllItem[] {
  const firstLvList = Object.values(data.firstLv)
    .filter((fl) => fl.categoryId === categoryId)
    .sort((a, b) => a.order - b.order);
  const result: AllItem[] = [];
  for (const fl of firstLvList) {
    const items = Object.values(data.allItems)
      .filter((i) => i.firstLvId === fl.firstLvId)
      .sort((a, b) => a.order - b.order);
    result.push(...items);
  }
  return result;
}

const pageOrder = ['multi_media', 'text', 'document'];
const pages = Object.values(data.PrtsPage).sort(
  (a, b) => pageOrder.indexOf(a.pageType) - pageOrder.indexOf(b.pageType),
);

const lines: string[] = [];
let riskTotal = 0;

for (const page of pages) {
  lines.push(`\n===== ${page.name} =====`);
  for (const catId of page.categoryIds) {
    const cat = data.PrtsCategory[catId];
    if (!cat) continue;
    const items = orderedItemsForCategory(catId);
    const titles = items.map((i) => i.title);
    lines.push(`\n--- ${cat.name}（${items.length}） ---`);
    for (const item of items) {
      const risks = detectRisks(item.title, titles);
      if (risks.length > 0) riskTotal++;
      const mark = risks.length > 0 ? `  ⚠ [${risks.join(' | ')}]` : '';
      lines.push(`  ${item.title}${mark}`);
    }
  }
}

fs.writeFileSync('temp/titles-inspection.txt', lines.join('\n') + '\n', 'utf8');
console.log(`共 ${riskTotal} 个标题有风险，完整列表已写入 temp/titles-inspection.txt`);

// 控制台只打印有风险的标题，便于快速浏览
console.log('\n===== 有风险的标题 =====');
for (const line of lines) {
  if (line.includes('⚠')) console.log(line);
}
