/**
 * 导出到地图集（OEM）的导入数据结构。
 *
 * 生成 upload.json 文本 → zip 压缩 → URL-safe Base64 → 作为
 * `https://oem.re/i/<base64>` 的路径。
 */
export interface UploadData {
  /** 数据格式主版本 */
  majorVersion: number;
  /** 数据格式次版本 */
  minorVersion: number;
  data: {
    /** 生成本数据的应用版本号 */
    oeaVersion: string;
    /** 全部档案的收集状态 */
    prtsAllItems: {
      /** 已收集档案 id 列表（allItems 的 id） */
      collected: string[];
      /** 未收集档案 id 列表（所有档案去掉已收集） */
      notCollected: string[];
    };
  };
}
