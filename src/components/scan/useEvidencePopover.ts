import { onUnmounted, ref, watch, type Ref } from 'vue';

// 点击展开。短暂延迟允许鼠标跨过按钮与弹窗之间的间隙。
export function useEvidencePopover(onClose: () => void): {
  open: Ref<boolean>;
  cancelClose: () => void;
  scheduleClose: () => void;
} {
  const open = ref(false);
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
    if (open.value) onClose();
  });
  return { open, cancelClose, scheduleClose };
}
