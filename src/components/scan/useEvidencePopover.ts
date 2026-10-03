import { onUnmounted, ref, watch, type Ref } from 'vue';

/**
 * 供两类证据弹窗共用。调用方通过点击控制 open，移开后延迟关闭，允许鼠标跨过弹窗间隙。
 * 普通关闭与来源 Card 卸载都会调用 onClose，用于清除对侧定位高亮。
 */
export function useEvidencePopover(onClose: () => void): {
  open: Ref<boolean>;
  cancelClose: () => void;
  scheduleClose: () => void;
} {
  const open: Ref<boolean> = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  function cancelClose(): void {
    clearTimeout(timer);
  }
  function scheduleClose(): void {
    cancelClose();
    timer = setTimeout((): void => {
      open.value = false;
    }, 150);
  }
  watch(open, (value: boolean): void => {
    if (!value) {
      cancelClose();
      onClose();
    }
  });
  onUnmounted((): void => {
    cancelClose();
    // 虚拟列表也会卸载 Card，只有仍打开的来源弹窗需要清理其定位高亮。
    if (open.value) onClose();
  });
  return { open, cancelClose, scheduleClose };
}
