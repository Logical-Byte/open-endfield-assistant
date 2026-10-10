import type {
  GameEnvironmentError,
  ScreenshotError,
  AutomationError,
  AutomationWorkerError,
  AutomationCapabilityError,
  UpdateError,
  UpdateErrorReason,
} from '@/shared/types/errors';
import { logError } from '@/features/log/ipc';
import { t } from '@/shared/i18n';

export type UpdateStage = 'check' | 'download' | 'install';

export type ErrorContext =
  | { operation: 'automation' }
  | { operation: 'screenshot' }
  | { operation: 'update'; stage: UpdateStage };
export type ErrorFacts =
  | { operation: 'automation'; error: AutomationError | null }
  | { operation: 'screenshot'; error: ScreenshotError | null }
  | { operation: 'update'; stage: UpdateStage; error: UpdateError | null };

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
function capabilityReason(value: unknown): value is AutomationCapabilityError {
  if (!record(value)) return false;
  switch (value.kind) {
    case 'gameEnvironment':
      return gameEnvironment(value.reason);
    case 'stoppedByUser':
    case 'captureFailed':
    case 'executionFailed':
      return true;
    default:
      return false;
  }
}
function workerReason(value: unknown): value is AutomationWorkerError {
  if (!record(value)) return false;
  switch (value.kind) {
    case 'capability':
      return capabilityReason(value.reason);
    case 'navigationFailed':
      return true;
    case 'custom':
      return typeof value.message === 'string';
    default:
      return false;
  }
}
function automationReason(value: unknown): value is AutomationError {
  if (!record(value)) return false;
  if (value.scope === 'runtime') {
    return value.kind === 'threadStartFailed';
  }
  return value.scope === 'worker' && workerReason(value);
}
function updateReason(value: unknown): value is UpdateErrorReason {
  if (!record(value)) return false;
  switch (value.kind) {
    case 'cancelled':
    case 'busy':
    case 'noUpdate':
    case 'proxyConfiguration':
    case 'network':
    case 'invalidMetadata':
    case 'integrity':
    case 'fileAccess':
    case 'debugBuild':
    case 'invalidPackage':
    case 'preparation':
    case 'helperStart':
    case 'failed':
      return true;
    case 'service':
      return typeof value.code === 'number' && Number.isSafeInteger(value.code);
    case 'versionMismatch':
      return typeof value.expected === 'string' && typeof value.actual === 'string';
    case 'packageUnavailable':
      return typeof value.version === 'string';
    default:
      return false;
  }
}
/** IPC 拒绝值只有验证形状与类型后才进入展示契约。未知值保留到日志。 */
export function normalizeBackendError(value: unknown, context: ErrorContext): ErrorFacts {
  switch (context.operation) {
    case 'automation':
      if (automationReason(value)) return { ...context, error: value };
      break;
    case 'screenshot':
      if (screenshotReason(value)) return { ...context, error: value };
      break;
    case 'update':
      if (record(value) && updateReason(value.reason)) {
        return { ...context, error: { reason: value.reason } };
      }
      break;
  }
  console.error('未知后端错误', { context, error: value });
  let diagnostic: string;
  try {
    diagnostic = typeof value === 'string' ? value : (JSON.stringify(value) ?? String(value));
  } catch {
    diagnostic = String(value);
  }
  void logError(`前端收到未知错误，操作 ${context.operation}: ${diagnostic}`).catch(
    (error: unknown) => {
      console.error('记录后端错误失败', error);
    },
  );
  return { ...context, error: null };
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
  if (!facts.error) {
    switch (facts.operation) {
      case 'automation':
        return t('errors.automation.failed');
      case 'screenshot':
        return t('errors.screenshot.failed');
      case 'update': {
        const keys = {
          check: 'errors.update.checkFailed',
          download: 'errors.update.downloadFailed',
          install: 'errors.update.installFailed',
        } as const;
        return t(keys[facts.stage]);
      }
    }
  }
  switch (facts.operation) {
    case 'automation': {
      const error = facts.error;
      if (error.scope === 'runtime') return t('errors.automation.threadStartFailed');
      switch (error.kind) {
        case 'capability':
          switch (error.reason.kind) {
            case 'gameEnvironment':
              return formatGameEnvironmentError(error.reason.reason);
            case 'captureFailed':
              return t('errors.automation.captureFailed');
            case 'executionFailed':
              return t('errors.automation.failed');
            case 'stoppedByUser':
              return t('errors.automation.stoppedByUser');
          }
        case 'custom':
          return error.message;
        case 'navigationFailed':
          return t('errors.automation.navigationFailed');
      }
    }
    case 'update':
      return formatUpdateError(facts.error.reason);
    case 'screenshot':
      switch (facts.error.kind) {
        case 'gameEnvironment':
          return formatGameEnvironmentError(facts.error.reason);
        case 'captureFailed':
          return t('errors.screenshot.captureFailed');
        case 'encodingFailed':
          return t('errors.screenshot.encodingFailed');
      }
  }
}

function formatUpdateError(reason: UpdateErrorReason): string {
  switch (reason.kind) {
    case 'cancelled':
      return t('errors.update.cancelled');
    case 'service': {
      const keys: Partial<Record<number, import('@/shared/i18n').MessageKey>> = {
        1001: 'errors.update.service.parameters',
        7001: 'errors.update.service.expired',
        7002: 'errors.update.service.invalidCdk',
        7003: 'errors.update.service.limit',
        7004: 'errors.update.service.cdkType',
        7005: 'errors.update.service.banned',
        8001: 'errors.update.service.resource',
        8002: 'errors.update.service.parameters',
        8003: 'errors.update.service.parameters',
        8004: 'errors.update.service.parameters',
      };
      const key = keys[reason.code];
      return key ? t(key) : t('errors.update.service.unknown', { code: reason.code });
    }
    case 'versionMismatch':
      return t('errors.update.versionMismatch', {
        expected: reason.expected,
        actual: reason.actual,
      });
    case 'packageUnavailable':
      return t('errors.update.packageUnavailable', { version: reason.version });
    case 'busy':
      return t('errors.update.busy');
    case 'noUpdate':
      return t('errors.update.noUpdate');
    case 'proxyConfiguration':
      return t('errors.update.proxyConfiguration');
    case 'network':
      return t('errors.update.network');
    case 'invalidMetadata':
      return t('errors.update.invalidMetadata');
    case 'integrity':
      return t('errors.update.integrity');
    case 'fileAccess':
      return t('errors.update.fileAccess');
    case 'debugBuild':
      return t('errors.update.debugBuild');
    case 'invalidPackage':
      return t('errors.update.invalidPackage');
    case 'preparation':
      return t('errors.update.preparation');
    case 'helperStart':
      return t('errors.update.helperStart');
    case 'failed':
      return t('errors.update.failed');
  }
}
