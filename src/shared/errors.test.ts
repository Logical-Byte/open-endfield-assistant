import { afterEach, expect, it, vi } from 'vitest';
import { computed } from 'vue';
import { formatBackendError, normalizeBackendError } from './errors';
import { i18n } from './i18n';

vi.mock('@/features/log/ipc', () => ({ logError: vi.fn().mockResolvedValue(undefined) }));

afterEach(async () => {
  await i18n.changeLanguage('zh-CN');
  vi.restoreAllMocks();
});

it('嵌套能力错误保留尺寸事实，已有错误随语言切换重新展示', async () => {
  await i18n.changeLanguage('zh-CN');
  const facts = normalizeBackendError(
    {
      scope: 'worker',
      kind: 'capability',
      reason: {
        kind: 'gameEnvironment',
        reason: { kind: 'unsupportedResolution', width: 1600, height: 1000 },
      },
    },
    { operation: 'automation' },
  );
  const message = computed(() => formatBackendError(facts));
  expect(message.value).toBe('游戏分辨率 1600×1000 不支持，请使用 16:9 分辨率。');
  await i18n.changeLanguage('en-US');
  expect(message.value).toBe(
    'The game resolution 1600×1000 is unsupported. Use a 16:9 resolution.',
  );
});

it('嵌套环境事实缺少尺寸时使用操作级提示，不插入无效参数', async () => {
  await i18n.changeLanguage('zh-CN');
  vi.spyOn(console, 'error').mockImplementation(() => {});
  const facts = normalizeBackendError(
    {
      scope: 'worker',
      kind: 'capability',
      reason: { kind: 'gameEnvironment', reason: { kind: 'unsupportedResolution', width: 1600 } },
    },
    { operation: 'automation' },
  );
  expect(formatBackendError(facts)).toBe('自动化任务失败，请查看日志。');
});
