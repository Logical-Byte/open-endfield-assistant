import type { BackendError, GameEnvironmentError, ScreenshotError } from '@/shared/types/errors';
import { logError } from '@/features/log/ipc';
import { t } from '@/shared/i18n';

export type BackendOperation = BackendError['operation'];
export type ErrorFacts = { operation: BackendOperation; error: BackendError | null };

type UnknownRecord = Record<string, unknown>;
function record(value: unknown): value is UnknownRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function dimension(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
}
function gameEnvironment(value: unknown): value is GameEnvironmentError {
  if (!record(value)) return false;
  switch (value.kind) {
    case 'windowUnavailable':
    case 'hdrEnabled':
      return true;
    case 'unsupportedResolution':
      return dimension(value.width) && dimension(value.height);
    case 'windowSizeChanged':
      return (
        dimension(value.expected_width) &&
        dimension(value.expected_height) &&
        dimension(value.actual_width) &&
        dimension(value.actual_height)
      );
    default:
      return false;
  }
}
function screenshotReason(value: unknown): value is ScreenshotError {
  if (!record(value)) return false;
  switch (value.kind) {
    case 'captureFailed':
    case 'encodingFailed':
      return true;
    case 'gameEnvironment':
      return gameEnvironment(value.reason);
    default:
      return false;
  }
}
/** IPC 拒绝值只有验证形状与类型后才进入展示契约。未知值保留到日志。 */
export function normalizeBackendError(value: unknown, operation: BackendOperation): ErrorFacts {
  if (record(value) && value.operation === operation && screenshotReason(value.reason)) {
    return { operation, error: value as BackendError };
  }
  console.error('未知后端错误', { operation, error: value });
  let diagnostic: string;
  try {
    diagnostic = typeof value === 'string' ? value : (JSON.stringify(value) ?? String(value));
  } catch {
    diagnostic = String(value);
  }
  void logError(`前端收到未知错误，操作 ${operation}: ${diagnostic}`).catch((error: unknown) => {
    console.error('记录后端错误失败', error);
  });
  return { operation, error: null };
}

export function formatGameEnvironmentError(reason: GameEnvironmentError): string {
  switch (reason.kind) {
    case 'windowUnavailable':
      return t('errors.game.windowUnavailable');
    case 'hdrEnabled':
      return t('errors.game.hdrEnabled');
    case 'unsupportedResolution':
      return t('errors.game.unsupportedResolution', { width: reason.width, height: reason.height });
    case 'windowSizeChanged':
      return t('errors.game.windowSizeChanged', {
        expectedWidth: reason.expected_width,
        expectedHeight: reason.expected_height,
        actualWidth: reason.actual_width,
        actualHeight: reason.actual_height,
      });
  }
}
/** 在展示时格式化事实，持续错误随 UiLocale 更新。 */
export function formatBackendError(facts: ErrorFacts): string {
  if (!facts.error) return t('errors.screenshot.failed');
  switch (facts.error.reason.kind) {
    case 'gameEnvironment':
      return formatGameEnvironmentError(facts.error.reason.reason);
    case 'captureFailed':
      return t('errors.screenshot.captureFailed');
    case 'encodingFailed':
      return t('errors.screenshot.encodingFailed');
  }
}
