/** 图片缩放配置选项 */
export interface UseImagePreviewScaleOptions {
  /** 最小缩放倍数 */
  minScale?: number;
  /** 最大缩放倍数 */
  maxScale?: number;
  /** 对数底数 */
  base?: number;
  /** 乘数数组 */
  coefficients?: number[];
  /** 滚轮缩放时的倍数 */
  multiplier?: number;
}

/**
 * 用于计算图片预览缩放倍数的组合式函数。
 *
 * 该函数提供了一系列工具，用于查找上一个或下一个“适当”的缩放倍数。
 * “适当”的缩放倍数定义为：`coefficient * base ^ n`，其中 `n` 为任意整数，`coefficient` 是 `coefficients` 数组中的任意元素。
 *
 * - `clampScale`: 用于将任意缩放值限制在 `minScale` 和 `maxScale` 之间。
 * - `getNextScale`: 用于寻找大于当前缩放值的最小“适当”缩放值。
 * - `getPrevScale`: 用于寻找小于当前缩放值的最大“适当”缩放值。
 * - `getNextScaleWithMultiplier`: 直接将当前缩放值乘以 `multiplier`，并限制在 `minScale` 和 `maxScale` 之间。
 * - `getPrevScaleWithMultiplier`: 直接将当前缩放值除以 `multiplier`，并限制在 `minScale` 和 `maxScale` 之间。
 *
 * @param options 缩放配置选项
 * @returns 缩放计算相关的工具函数
 */
export function useImagePreviewScale(options: UseImagePreviewScaleOptions = {}) {
  const {
    minScale = 1 / 1024,
    maxScale = 1024,
    base = 2,
    coefficients = [1, 1.25, 1.5],
    multiplier = 1.25,
  } = options;

  /** 乘数数组正序和反序 */
  const coefficientSorted = [...coefficients].sort((a, b) => a - b);
  const coefficientReversed = [...coefficientSorted].reverse();

  /** 将缩放倍数限制在允许的范围内 */
  function clampScale(value: number): number {
    return Math.min(maxScale, Math.max(minScale, value));
  }

  /** 获取下一个步进缩放倍数，即找到第一个大于当前值的缩放倍数 */
  function getNextScale(value: number): number {
    const exponent = Math.floor(Math.log(value) / Math.log(base));

    for (const exp of [exponent, exponent + 1]) {
      const baseExp = base ** exp;
      for (const coefficient of coefficientSorted) {
        const candidate = baseExp * coefficient;
        if (candidate > value) {
          return clampScale(candidate);
        }
      }
    }

    return maxScale;
  }

  /** 获取上一个步进缩放倍数，即找到第一个小于当前值的缩放倍数 */
  function getPrevScale(value: number): number {
    const exponent = Math.floor(Math.log(value) / Math.log(base));

    for (const exp of [exponent, exponent - 1]) {
      const baseExp = base ** exp;
      for (const coefficient of coefficientReversed) {
        const candidate = baseExp * coefficient;
        if (candidate < value) {
          return clampScale(candidate);
        }
      }
    }

    return minScale;
  }

  /** 用倍数计算下一个缩放倍数，直接乘以 multiplier */
  function getNextScaleWithMultiplier(value: number): number {
    return clampScale(value * multiplier);
  }

  /** 用倍数计算上一个缩放倍数，直接除以 multiplier */
  function getPrevScaleWithMultiplier(value: number): number {
    return clampScale(value / multiplier);
  }

  return {
    clampScale,
    getNextScale,
    getPrevScale,
    getNextScaleWithMultiplier,
    getPrevScaleWithMultiplier,
  };
}
