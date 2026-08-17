export interface OeaConfig {
  /** 配置文件版本号（用于升级时迁移配置） */
  version: [number, number];
  /** 关闭时最小化到托盘而不是退出应用 */
  minimizeToTray: boolean;
  /** 扫描音效音量（0.0–1.0） */
  soundVolume: number;
  /** 启动时检查新版本。 */
  checkUpdates: boolean;
  updateSource: 'mirrorchyan' | 'github';
  mirrorchyanCdk: string;
  updateProxyMode: 'none' | 'system' | 'custom';
  updateProxyUrl: string;
}
