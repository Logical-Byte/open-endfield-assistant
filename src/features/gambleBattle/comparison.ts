import type { Comparison } from '@/shared/types/gambleBattle';

export const comparisonSymbols: Record<Comparison, string> = {
  greaterOrEqual: '≥',
  greater: '>',
  equal: '=',
  less: '<',
  lessOrEqual: '≤',
};

/** Keep strict comparisons within the possible total BP range. */
export function sumThresholdRange(comparison: Comparison): { min: number; max: number } {
  return { min: comparison === 'less' ? 2 : 1, max: comparison === 'greater' ? 24 : 25 };
}
