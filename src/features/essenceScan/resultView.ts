import type { DeepReadonly } from 'vue';
import type {
  Decision,
  Essence,
  EssenceCatalog,
  Evaluation,
  MarkingOutcome,
  ScannedItem,
} from './types';

interface DecisionPresentation {
  label: string;
  color: 'success' | 'error' | 'neutral' | 'warning';
  icon: string;
}

export const decisionPresentation: Record<Decision, DecisionPresentation> = {
  keep: { label: '保留', color: 'success', icon: 'i-lucide-shield-check' },
  discard: { label: '丢弃', color: 'error', icon: 'i-lucide-trash-2' },
  skip: { label: '跳过', color: 'neutral', icon: 'i-lucide-skip-forward' },
  review: { label: '待确认', color: 'warning', icon: 'i-lucide-scan-eye' },
};

export const reasonLabels: Record<Evaluation['reason'], string> = {
  locked: '基质已锁定，按保护规则保留',
  abandoned: '基质已标记弃用，按设置跳过',
  nonFiveStar: '非五星基质，按设置跳过',
  customRule: '符合自定义保留组合',
  highLevel: '至少一个词条达到高等级保留阈值',
  weaponMatch: '匹配到未排除的武器',
  excludedWeapons: '匹配到的武器均已被排除',
  noMatchingWeapon: '没有匹配的武器，也未命中其他保留规则',
  incompleteRecognition: '识别信息不完整，请查看截图确认',
};

export const rarityLabels: Record<Essence['rarity'], string> = {
  five: '五星',
  four: '四星',
  other: '其他星级',
  unknown: '星级未识别',
};

/** 操作反馈独立于保留判断，模拟结果始终保留模拟标识。 */
export function markingPresentation(marking: MarkingOutcome): DecisionPresentation {
  switch (marking.status) {
    case 'disabled':
      return { label: '仅扫描', color: 'neutral', icon: 'i-lucide-scan-line' };
    case 'skipped':
      return { label: '未修改标记', color: 'neutral', icon: 'i-lucide-minus' };
    case 'alreadySet':
      return {
        label: marking.action === 'lock' ? '原已锁定' : '原已标记弃用',
        color: 'neutral',
        icon: 'i-lucide-check',
      };
    case 'applied':
      return {
        label: marking.action === 'lock' ? '已锁定并确认' : '已标记弃用并确认',
        color: 'success',
        icon: 'i-lucide-check-check',
      };
    case 'simulated':
      return {
        label: marking.action === 'lock' ? '模拟锁定' : '模拟标记弃用',
        color: 'warning',
        icon: 'i-lucide-flask-conical',
      };
    case 'failed':
      return {
        label: marking.action === 'lock' ? '锁定未确认' : '弃用未确认',
        color: 'error',
        icon: 'i-lucide-circle-alert',
      };
  }
}

export interface ScanSummary {
  total: number;
  keep: number;
  discard: number;
  skip: number;
  review: number;
  marking: { applied: number; simulated: number; failed: number };
}

export interface ResultFilters {
  decision: 'all' | Decision;
  query: string;
}

/** 统计后端给出的结论，修改当前规则不会重算历史结果。 */
export function deriveSummary(items: readonly DeepReadonly<ScannedItem>[]): ScanSummary {
  const summary: ScanSummary = {
    total: items.length,
    keep: 0,
    discard: 0,
    skip: 0,
    review: 0,
    marking: { applied: 0, simulated: 0, failed: 0 },
  };
  for (const item of items) {
    summary[item.evaluation.decision] += 1;
    const status = item.marking.status;
    if (status === 'applied' || status === 'simulated' || status === 'failed')
      summary.marking[status] += 1;
  }
  return summary;
}

export function filterItems(
  items: readonly DeepReadonly<ScannedItem>[],
  catalog: DeepReadonly<EssenceCatalog> | null,
  filters: ResultFilters,
): DeepReadonly<ScannedItem>[] {
  const query = filters.query.trim().toLocaleLowerCase();
  const stats = new Map(catalog?.stats.map((stat) => [stat.id, stat.name]));
  const weapons = new Map(catalog?.weapons.map((weapon) => [weapon.id, weapon.name]));
  return items.filter((item): boolean => {
    if (filters.decision !== 'all' && item.evaluation.decision !== filters.decision) return false;
    if (query.length === 0) return true;
    const searchable = [
      String(item.sequence),
      ...item.essence.stats.map((id) => (id === null ? '' : (stats.get(id) ?? id))),
      ...item.evaluation.matchedWeaponIds.map((id) => weapons.get(id) ?? id),
    ];
    return searchable.some((text: string): boolean => text.toLocaleLowerCase().includes(query));
  });
}
