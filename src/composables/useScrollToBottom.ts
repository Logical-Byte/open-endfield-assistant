import type { Ref, WatchSource } from 'vue';
import { nextTick, onMounted, watch } from 'vue';

/** 是否已滚动到底部（用于判断是否跟随新日志自动滚动）。 */
export function useScrollToBottom(
  logContainerRef: Ref<HTMLElement | null>,
  filteredLogLines: WatchSource,
) {
  function isAtBottom(): boolean {
    const el = logContainerRef.value;
    if (!el) {
      return true;
    }
    return el.scrollTop + el.clientHeight >= el.scrollHeight - 4;
  }

  function scrollToBottom(): void {
    const el = logContainerRef.value;
    if (el) {
      el.scrollTo({ top: el.scrollHeight });
    }
  }

  // 挂载时滚动到底部，直接展示最新日志
  onMounted(() => {
    nextTick(scrollToBottom);
  });

  // 新日志（或过滤等级变化）到来时，若用户已在底部则自动跟随滚动
  watch(filteredLogLines, () => {
    if (isAtBottom()) {
      requestAnimationFrame(scrollToBottom);
    }
  });
}
