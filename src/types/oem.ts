/** OEM 稳定版元数据响应。 */
export interface OemStableManifest {
  schemaVersion: 1;
  channel: 'stable';
  version: string;
  tag: string;
  filename: string;
  key: string;
  url: string;
  size: number;
  sha256: string;
  publishedAt: string;
}
