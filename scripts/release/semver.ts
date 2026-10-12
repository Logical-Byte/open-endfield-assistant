const STRICT_SEMVER_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

/** 校验项目发布流程使用的纯数字 SemVer。 */
export function parseStrictSemver(value: string): [number, number, number] | null {
  const match = value.match(STRICT_SEMVER_PATTERN);
  if (!match) {
    return null;
  }
  const parts: [number, number, number] = [Number(match[1]), Number(match[2]), Number(match[3])];
  return parts.every(Number.isSafeInteger) ? parts : null;
}

/** 比较两个已经通过严格校验的项目版本号。 */
export function compareStrictSemver(left: string, right: string): number {
  const leftParts = parseStrictSemver(left);
  const rightParts = parseStrictSemver(right);
  if (!leftParts || !rightParts) {
    throw new Error(`无法比较无效版本号: ${left} / ${right}`);
  }
  for (let index = 0; index < leftParts.length; index += 1) {
    const difference = leftParts[index] - rightParts[index];
    if (difference !== 0) {
      return difference;
    }
  }
  return 0;
}
