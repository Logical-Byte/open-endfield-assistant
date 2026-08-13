export enum UpdateSource {
  Mirrorchyan = 'mirrorchyan',
  Github = 'github',
}

export enum UpdateProxyMode {
  System = 'system',
  Direct = 'direct',
  Proxy = 'proxy',
}

export interface OeaConfig {
  majorVersion: number;
  minorVersion: number;
  minimizeToTray: boolean;
  soundVolume: number;
  updateSource: UpdateSource;
  mirrorchyanCdkEncrypted: string | null;
  updateProxyMode: UpdateProxyMode;
  updateProxyUrl: string | null;
}
