/**
 * 统一数据生成入口脚本
 * 依次运行所有 make* 任务，生成全部数据文件。
 * 执行方式: `pnpm makedata`
 */
import fs from 'node:fs';
import { makePrts } from './tasks/makePrts';

function main() {
  fs.writeFileSync('resources/data/prts.json', JSON.stringify(makePrts(), null, 2), 'utf8');
}

main();
