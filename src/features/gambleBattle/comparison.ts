import type { Comparison } from '@/shared/types/gambleBattle';

export const comparisonSymbols: Record<Comparison, string> = {
  greaterOrEqual: '≥',
  greater: '>',
  equal: '=',
  less: '<',
  lessOrEqual: '≤',
};
