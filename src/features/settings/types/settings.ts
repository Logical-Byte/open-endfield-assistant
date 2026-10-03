import type { EssenceScanSettings } from '@/features/essenceScan/types';

export enum UpdateSource {
  Mirrorchyan = 'mirrorchyan',
  Oem = 'oem',
  Github = 'github',
}

export enum UpdateProxyMode {
  None = 'none',
  System = 'system',
  Custom = 'custom',
}

export interface Settings {
  /** 设置文件主版本；字段结构发生不兼容变化时递增。 */
  majorVersion: number;
  /** 设置文件次版本；添加兼容字段时递增。 */
  minorVersion: number;
  /** 关闭窗口时是否隐藏到系统托盘，而非退出应用。 */
  minimizeToTray: boolean;
  /** 扫描提示音量，范围为 `0` 至 `1`。 */
  soundVolume: number;
  /** 下载应用更新时使用的更新源。 */
  updateSource: UpdateSource;
  /** Mirror酱 CDK 密文；前端设置 store 将其投影为可编辑的明文。 */
  mirrorchyanCdkEncrypted: string;
  /** 下载应用更新时使用的代理模式。 */
  updateProxyMode: UpdateProxyMode;
  /** 自定义更新代理 URL，仅在 `updateProxyMode` 为 `Custom` 时使用。 */
  updateProxyUrl: string;
  /** 检查到新版本后是否自动开始下载。 */
  autoDownloadUpdates: boolean;
  /** 下载完成后是否自动安装（扫描任务运行中不会安装，等待扫描结束）。 */
  autoInstallUpdates: boolean;
  /** 用户已确认的扫描启动提示版本；设置 store 将其投影为 `scanGuideEnabled`。 */
  scanTipsDismissedVersion: number;
  /** 基质扫描的独立保留规则。 */
  essenceScan: EssenceScanSettings;
}
