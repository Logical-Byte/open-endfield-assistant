import type { TranslationKey } from './common';

export interface BattlePassTrack {
  bpExpUpRatio: number;
  name: TranslationKey;
  trackId: string;
  trackType: number;
}

export type BattlePassTrackTable = Record<string, BattlePassTrack>;
