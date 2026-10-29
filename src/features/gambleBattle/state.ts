import { reactive, readonly, ref, shallowRef, watch, type DeepReadonly, type Ref } from 'vue';
import type { Conditions, HandKind, MatchedHand, Progress } from '@/shared/types/gambleBattle';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import { onGambleBattleMatched, onGambleBattleProgress } from './ipc';

/** 按游戏的牌型优先级排列，也是条件编辑器的展示顺序。 */
export const handKinds: HandKind[] = [
  'straightFlush',
  'fourOfAKind',
  'flush',
  'fullHouse',
  'straight',
  'threeOfAKind',
  'twoPairs',
  'pair',
];

// 一次启动参数在页面切换后继续保留，不写入应用的持久化设置。
export const conditions = reactive<Conditions>({
  straightFlush: true,
  fourOfAKind: false,
  flush: false,
  fullHouse: false,
  straight: false,
  threeOfAKind: false,
  twoPairs: false,
  pair: false,
  sum: null,
});
export const sumThreshold = shallowRef(25);
/** 教程只在本次应用会话内保留显示状态。 */
export const showGambleBattleGuide = ref(true);
const matched = shallowRef<MatchedHand[]>([]);
export const matchedHands: Readonly<Ref<DeepReadonly<MatchedHand[]>>> = readonly(matched);
export const expandedMatches = ref<Record<number, boolean>>({});

export function clearMatchedHands(): void {
  matched.value = [];
  expandedMatches.value = {};
}

const progress = shallowRef<Progress>({ round: 0, elapsedSeconds: 0 });
const progressReceivedAt = shallowRef(Date.now());
export const currentProgress = readonly(progress);
export const progressUpdatedAt = readonly(progressReceivedAt);

/** 在应用层监听，离开活动页面后仍能接收成功结果。 */
export async function initGambleBattleState(): Promise<void> {
  const task = useAutomationTask('gambleBattle');
  watch(
    task.phase,
    (phase, previousPhase) => {
      if (phase === 'running') {
        progress.value = { round: 1, elapsedSeconds: 0 };
        progressReceivedAt.value = Date.now();
      } else if (
        phase === 'idle' &&
        (previousPhase === 'running' || previousPhase === 'stopping')
      ) {
        progress.value = {
          ...progress.value,
          elapsedSeconds:
            progress.value.elapsedSeconds + (Date.now() - progressReceivedAt.value) / 1000,
        };
      }
    },
    { flush: 'sync' },
  );
  await onGambleBattleProgress((nextProgress: Progress) => {
    progress.value = nextProgress;
    progressReceivedAt.value = Date.now();
  });
  await onGambleBattleMatched((result: MatchedHand) => {
    matched.value = [...matched.value, result];
    progress.value = { round: result.round, elapsedSeconds: result.elapsedSeconds };
    progressReceivedAt.value = Date.now();
  });
}
