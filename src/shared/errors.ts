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

/** 先适配结构化错误契约，详细原因翻译在下一层接入。 */
export function formatBackendError(facts: ErrorFacts): string {
  switch (facts.operation) {
    case 'automation':
      return '自动化任务失败，请查看日志。';
    case 'screenshot':
      return '截图失败，请查看日志。';
    case 'update':
      switch (facts.stage) {
        case 'check':
          return '检查更新失败，请查看日志。';
        case 'download':
          return '下载更新失败，请查看日志。';
        case 'install':
          return '安装更新失败，请查看日志。';
      }
  }
}
