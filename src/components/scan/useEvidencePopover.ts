import { onUnmounted, ref, watch, type Ref } from 'vue';

/**
 * 点击控制 open，移开后延迟关闭，允许鼠标跨过弹窗间隙。
 * 定位高亮由列表独立计时，不依赖弹窗是否关闭。
 */
export function useEvidencePopover(): {
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
    }
  });
  onUnmounted(cancelClose);
  return { open, cancelClose, scheduleClose };
}
