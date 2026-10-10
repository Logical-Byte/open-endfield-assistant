import { watch } from 'vue';
type Toast = ReturnType<typeof useToast>['toasts']['value'][number];
import { i18n } from './index';

export type ToastText = Pick<Partial<Toast>, 'title' | 'description' | 'actions'>;
const textSources = new Map<
  Toast['id'],
  { render: () => ToastText; queued: Toast; seen: boolean }
>();

/** 在应用根安装一次。只改文字字段，避免 Nuxt update 重开通知和重置计时。 */
export function bindTranslatedToasts(): void {
  const toast = useToast();
  watch(i18n.global.locale, () => {
    for (const [id, source] of textSources) {
      const current =
        toast.toasts.value.find((item: Toast): boolean => item.id === id) ?? source.queued;
      if (current.open === false) continue;
      const text = source.render();
      current.title = text.title;
      current.description = text.description;
      if (current.actions && text.actions) {
        current.actions = current.actions.map(
          (
            action: NonNullable<Toast['actions']>[number],
            index: number,
          ): NonNullable<Toast['actions']>[number] => ({
            ...action,
            label: text.actions?.[index]?.label ?? action.label,
          }),
        );
      }
    }
  });
  watch(
    toast.toasts,
    (visible: Toast[]) => {
      const ids = new Set(visible.map((item: Toast): Toast['id'] => item.id));
      for (const [id, source] of textSources) {
        if (ids.has(id)) source.seen = true;
        // Nuxt add 先进入异步队列，尚未展示的翻译来源需要保留。
        else if (source.seen) textSources.delete(id);
      }
    },
    { deep: true },
  );
}

/** 保留事实和回调，显示文案在首次添加和语言切换时生成。 */
export function useTranslatedToast(): {
  add: (options: Partial<Toast>, render: () => ToastText) => Toast;
} {
  const toast = useToast();
  function add(options: Partial<Toast>, render: () => ToastText): Toast {
    const added = toast.add({ ...options, ...render() });
    textSources.set(added.id, { render, queued: added, seen: false });
    return added;
  }
  return { add };
}
