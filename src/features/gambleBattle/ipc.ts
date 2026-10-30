import { listen, type Event } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { DailyLucky, MatchedHand, Progress } from '@/shared/types/gambleBattle';

/** 今日好运组合与各牌型倍率，换日后重新查询。 */
export async function getGambleBattleDaily(): Promise<DailyLucky> {
  return await invoke('get_gamble_battle_daily');
}

/** 工作流已经命中目标并保留当前五张牌时发布。 */
export async function onGambleBattleMatched(
  callback: (result: MatchedHand) => void,
): Promise<() => void> {
  return await listen<MatchedHand>('gamble-battle-matched', (event: Event<MatchedHand>) =>
    callback(event.payload),
  );
}

export async function onGambleBattleProgress(
  callback: (progress: Progress) => void,
): Promise<() => void> {
  return await listen<Progress>('gamble-battle-progress', (event: Event<Progress>) =>
    callback(event.payload),
  );
}

/** 全局分号快捷键，由应用层使用当前停止条件处理。 */
export async function onGambleBattleToggleRequested(
  callback: () => Promise<void>,
): Promise<() => void> {
  return await listen('gamble-battle-toggle-requested', callback);
}
