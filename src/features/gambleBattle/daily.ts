import { readonly, shallowRef, type DeepReadonly, type Ref } from 'vue';
import type { DailyLucky } from '@/shared/types/gambleBattle';
import { logError } from '@/features/log/ipc';
import { getGambleBattleDaily } from './ipc';

const daily = shallowRef<DailyLucky | null>(null);
/** 今日好运组合。查询失败时为 `null`，界面只隐藏相关标注。 */
export const gambleBattleDaily: Readonly<Ref<DeepReadonly<DailyLucky | null>>> = readonly(daily);

let timer: ReturnType<typeof setTimeout> | undefined;

/** 查询今日好运组合，并在下一次换日后自动重新查询。 */
export async function refreshGambleBattleDaily(): Promise<void> {
  clearTimeout(timer);
  try {
    daily.value = await getGambleBattleDaily();
  } catch (error: unknown) {
    daily.value = null;
    logError(`读取百趣演武好运组合失败：${String(error)}`);
    // 失败后稍后重试，避免一次失败就让标注一直缺失。
    timer = setTimeout((): void => void refreshGambleBattleDaily(), 60_000);
    return;
  }
  // 多等一秒，避免刚好卡在换日界上取到旧值。
  const delay = (daily.value.nextChangeInSeconds + 1) * 1000;
  timer = setTimeout((): void => void refreshGambleBattleDaily(), delay);
}
