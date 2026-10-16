/**
 * 统一档案数据生成入口，在仓库根目录运行：pnpm makedata。
 * 通过环境变量或根目录 .env 的 ENDFIELD_DATA_DIR 指定解包数据根目录（TableCfg 的父目录）。
 * 输入包括 TableCfg 中的档案表和 CN/EN 文本，以及 Json 下的任务、关卡和 NPC 配置。
 * 依次生成并覆盖 resources/data/prts.json 和 resources/data/archive_contract.json。
 */
import fs from 'node:fs';
import { exportArchiveContract } from './tasks/exportArchiveContract';
import { makePrts } from './tasks/makePrts';

function main() {
  fs.writeFileSync('resources/data/prts.json', JSON.stringify(makePrts(), null, 2), 'utf8');
  fs.writeFileSync(
    'resources/data/archive_contract.json',
    `${JSON.stringify(exportArchiveContract(), null, 2)}\n`,
    'utf8',
  );
}

main();
