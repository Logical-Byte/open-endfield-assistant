import { computed, ref, type ComputedRef, type Ref } from 'vue';
import { useAutomationTask, type AutomationTask } from '@/features/automation/useAutomationTask';
import { bpPattern, conditions, handKinds, scoreThreshold, sumThreshold } from './state';
import { gambleBattleWorkerType } from './workerType';
import { useAppI18n } from '@/shared/i18n';
import { useTranslatedToast } from '@/shared/i18n/toast';
import { formatBackendError, normalizeBackendError } from '@/shared/errors';

// 页面按钮和全局快捷键共用提交锁，避免 IPC 返回前重复切换。
const submitting = ref(false);

export function useGambleBattleControls(): {
  task: AutomationTask<'gambleBattle'>;
  submitting: Readonly<Ref<boolean>>;
  hasCondition: ComputedRef<boolean>;
  sumValid: ComputedRef<boolean>;
  scoreValid: ComputedRef<boolean>;
  patternValid: ComputedRef<boolean>;
  toggleAutomation: () => Promise<void>;
} {
  const task = useAutomationTask('gambleBattle');
  const { canStart, tryStart, tryStop } = task;
  const { t } = useAppI18n();
  const toast = useTranslatedToast();
  const hasCondition = computed(
    (): boolean =>
      handKinds.some((hand): boolean => conditions[hand]) ||
      conditions.sum !== null ||
      conditions.score !== null ||
      conditions.pattern !== null,
  );
  const sumValid = computed(
    (): boolean =>
      conditions.sum === null ||
      (Number.isInteger(sumThreshold.value) && sumThreshold.value >= 1 && sumThreshold.value <= 25),
  );
  const scoreValid = computed(
    (): boolean =>
      conditions.score === null ||
      (Number.isInteger(scoreThreshold.value) && scoreThreshold.value > 0),
  );
  const patternValid = computed(
    (): boolean => conditions.pattern === null || /^[1-5]{1,5}$/.test(bpPattern.value),
  );
  const inputsValid = computed(
    (): boolean => sumValid.value && scoreValid.value && patternValid.value,
  );
  async function toggleAutomation(): Promise<void> {
    if (submitting.value) return;
    submitting.value = true;
    try {
      if (canStart.value) {
        if (!hasCondition.value || !inputsValid.value) return;
        await tryStart({ conditions: { ...conditions }, workerType: gambleBattleWorkerType.value });
      } else {
        await tryStop();
      }
    } catch (error: unknown) {
      const facts = normalizeBackendError(error, { operation: 'automation' });
      toast.add({ color: 'error' }, () => ({
        title: t('gambleBattle.failed'),
        description: formatBackendError(facts),
      }));
    } finally {
      submitting.value = false;
    }
  }
  return { task, submitting, hasCondition, sumValid, scoreValid, patternValid, toggleAutomation };
}
