import {
  computed,
  reactive,
  readonly,
  ref,
  shallowRef,
  watch,
  type DeepReadonly,
  type Ref,
} from 'vue';
import type {
  Comparison,
  Conditions,
  HandKind,
  MatchedHand,
  OtherCondition,
  OtherConditions,
  Progress,
} from '@/shared/types/gambleBattle';
import { useAutomationTask } from '@/features/automation/useAutomationTask';
import { refreshGambleBattleDaily } from './daily';
import { onGambleBattleMatched, onGambleBattleProgress } from './ipc';

/** 按游戏的牌型优先级排列，也是条件编辑器的展示顺序。 */
export const handKinds: HandKind[] = [
  'straightFlush',
  'quad',
  'flush',
  'fullHouse',
  'straight',
  'triplet',
  'twoPair',
  'pair',
];

// 一次启动参数在页面切换后继续保留，不写入应用的持久化设置。
// 编辑状态允许关闭全部条件，执行 AST 则始终由非空条件构造。
type ConditionDraft = Record<HandKind, boolean> & {
  sum: number | null;
  score: number | null;
  pattern: string | null;
};
export const conditions = reactive<ConditionDraft>({
  straightFlush: true,
  quad: false,
  flush: false,
  fullHouse: false,
  straight: false,
  triplet: false,
  twoPair: false,
  pair: false,
  sum: null,
  score: null,
  pattern: null,
});
export const allowEarlyStop = ref(false);
export const sumThreshold = shallowRef(20);
export const scoreThreshold = shallowRef(900000);
export const bpPattern = shallowRef('325');
export const sumComparison = ref<Comparison>('greaterOrEqual');
export const sumJoin = ref<'and' | 'or'>('or');
const otherConditions = computed((): OtherConditions | null => {
  const items: OtherCondition[] = handKinds
    .filter((hand: HandKind): boolean => conditions[hand])
    .map((hand: HandKind): OtherCondition => ({ kind: 'hand', hand }));
  if (conditions.score !== null) items.push({ kind: 'score', threshold: scoreThreshold.value });
  if (conditions.pattern !== null) items.push({ kind: 'pattern', pattern: bpPattern.value });
  const [first, ...rest] = items;
  return first ? { first, rest } : null;
});
export const hasOtherConditions = computed((): boolean => otherConditions.value !== null);
export const executionConditions = computed((): Conditions | null => {
  const other = otherConditions.value;
  const sum =
    conditions.sum !== null
      ? { comparison: sumComparison.value, threshold: sumThreshold.value }
      : null;
  if (other && sum) return { kind: sumJoin.value, other, sum };
  if (sum) return { kind: 'sum', condition: sum };
  return other ? { kind: 'other', conditions: other } : null;
});
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
  await refreshGambleBattleDaily();
}
