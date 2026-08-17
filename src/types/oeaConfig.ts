export interface OeaConfig {
  /** 配置文件版本号（用于升级时迁移配置） */
  version: [number, number];
  /** 关闭时最小化到托盘而不是退出应用 */
  minimizeToTray: boolean;
  /** 扫描音效音量（0.0–1.0） */
  soundVolume: number;
  /** 启动时检查新版本。 */
  checkUpdates: boolean;
  /** 更新来源；该产品选项改编自 PR #6。 */
  updateSource: 'mirrorchyan' | 'github';
  /** MirrorChyan 凭据；留空时后端使用 GitHub。 */
  mirrorchyanCdk: string;
  /** 更新 HTTP 请求的代理策略；该产品选项改编自 PR #6。 */
  updateProxyMode: 'none' | 'system' | 'custom';
  /** 自定义模式下的完整代理 URL。 */
  updateProxyUrl: string;
}
