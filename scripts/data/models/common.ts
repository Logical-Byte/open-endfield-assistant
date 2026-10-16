/** 解包数据中的多语言键值对 */
export interface TranslationKey {
  id: string | number; // 在解包数据中是 64 位有符号整数，解析时转换为 string 避免精度丢失
  text: string;
}

/** 三维向量（关卡实体位置 / 旋转 / 缩放） */
export interface Vector3 {
  x: number;
  y: number;
  z: number;
}
