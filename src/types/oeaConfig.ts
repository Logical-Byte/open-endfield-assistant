export interface OeaConfig {
  /** 配置文件版本号（用于升级时迁移配置） */
  version: [number, number];
  /** 关闭时最小化到托盘而不是退出应用 */
  minimizeToTray: boolean;
}
