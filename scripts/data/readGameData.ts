/**
 * 解包数据读取模块，供数据生成任务导入，不直接运行。
 * 导入时加载仓库根目录的 .env，并要求 ENDFIELD_DATA_DIR 指向 TableCfg 的父目录。
 * 读取函数同步解析该目录下的 JSON，将数值 id 保留为原始 token 字符串以避免精度损失。
 */
import dotenv from 'dotenv';
import fs from 'node:fs';
import path from 'node:path';

dotenv.config();
if (!process.env.ENDFIELD_DATA_DIR) {
  throw new Error('请设置 ENDFIELD_DATA_DIR，指向 TableCfg 的父目录。');
}
export const endfieldDataDir = process.env.ENDFIELD_DATA_DIR;

/** JSON.parse 的原始 token 保留 TranslationKey 的整数身份，避免数值舍入。 */
export function parseJSONWithBigInt<T>(text: string): T {
  return JSON.parse(text, (key, value, context?: { source: string }) =>
    key === 'id' && typeof value === 'number' ? context!.source : value,
  );
}

export function readJSONWithBigInt<T>(relativePath: string): T {
  return parseJSONWithBigInt<T>(fs.readFileSync(path.join(endfieldDataDir, relativePath), 'utf8'));
}
