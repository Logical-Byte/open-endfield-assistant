import { ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';

import { openUpdateSettings } from './updatePopoverNavigation';

describe('更新弹窗设置入口', () => {
  it('关闭弹窗并跳转至更新设置区域', () => {
    const popoverOpen = ref(true);
    const push = vi.fn(() => Promise.resolve());

    openUpdateSettings(popoverOpen, { push });

    expect(popoverOpen.value).toBe(false);
    expect(push).toHaveBeenCalledWith('/settings#update');
  });
});
