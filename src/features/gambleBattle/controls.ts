import { computed, ref, type ComputedRef, type Ref } from 'vue';
import { useAutomationTask, type AutomationTask } from '@/features/automation/useAutomationTask';
import { conditions, handKinds, sumThreshold } from './state';
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
  thresholdValid: ComputedRef<boolean>;
  toggleAutomation: () => Promise<void>;
} {
  const task = useAutomationTask('gambleBattle');
  const { canStart, tryStart, tryStop } = task;
  const { t } = useAppI18n();
  const toast = useTranslatedToast();
  const hasCondition = computed(
    (): boolean => handKinds.some((hand): boolean => conditions[hand]) || conditions.sum !== null,
  );
  const thresholdValid = computed(
    (): boolean =>
      conditions.sum === null ||
      (Number.isInteger(sumThreshold.value) && sumThreshold.value >= 1 && sumThreshold.value <= 25),
  );
  async function toggleAutomation(): Promise<void> {
    if (submitting.value) return;
    submitting.value = true;
    try {
      if (canStart.value) {
        if (!hasCondition.value || !thresholdValid.value) return;
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
  return { task, submitting, hasCondition, thresholdValid, toggleAutomation };
}
