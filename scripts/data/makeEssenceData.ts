import dotenv from 'dotenv';
import { readEssenceTables, writeEssence } from './tasks/makeEssence';

dotenv.config();
if (!process.env.ENDFIELD_DATA_DIR) {
  throw new Error('请设置 ENDFIELD_DATA_DIR，指向 TableCfg 文件夹的父目录。');
}
writeEssence(readEssenceTables(process.env.ENDFIELD_DATA_DIR));
