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

export interface OeaConfig {
  majorVersion: number;
  minorVersion: number;
  minimizeToTray: boolean;
  soundVolume: number;
  updateSource: UpdateSource;
  mirrorchyanCdkEncrypted: string;
  updateProxyMode: UpdateProxyMode;
  updateProxyUrl: string;
  /** 检查到新版本后是否自动开始下载。 */
  autoDownloadUpdates: boolean;
  /** 下载完成后是否自动安装（扫描任务运行中不会安装，等待扫描结束）。 */
  autoInstallUpdates: boolean;
  scanTipsDismissedVersion: number;
}
