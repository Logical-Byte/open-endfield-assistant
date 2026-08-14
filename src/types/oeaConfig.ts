export enum UpdateSource {
  Mirrorchyan = 'mirrorchyan',
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
  scanTipsDismissedVersion: number;
}
