import { describe, expect, it } from 'vitest';

import { uiScale } from './uiScale';

describe('UI 缩放', () => {
  it('拒绝非有限值，并将有限值限制在支持范围内', () => {
    const initialScale = uiScale.value;

    try {
      uiScale.value = 1;
      (uiScale as { value: unknown }).value = Number.NaN;
      (uiScale as { value: unknown }).value = Number.POSITIVE_INFINITY;
      expect(uiScale.value).toBe(1);

      uiScale.value = 3;
      expect(uiScale.value).toBe(2);

      uiScale.value = 0.1;
      expect(uiScale.value).toBe(0.5);
    } finally {
      uiScale.value = initialScale;
    }
  });
});
