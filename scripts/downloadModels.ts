/**
 * 模型下载脚本：将 OCR 所需的模型文件下载到 <root>/resources/ocr-models/。
 *
 * 模型文件体积较大且已被 git 忽略（见 resources/ocr-models/.gitignore），克隆仓库后需要先执行：
 *   pnpm download:models
 *
 * 已存在的文件会被跳过，可用 `--force` 强制重新下载。
 *
 * 执行方式: `pnpm download:models` 或 `pnpm download:models --force`
 */
import { createWriteStream, existsSync, mkdirSync, renameSync } from 'node:fs';
import path from 'node:path';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';

interface ModelFile {
  /** ModelScope 下载地址 */
  url: string;
  /** 保存到 resources/ocr-models/ 下的文件名 */
  name: string;
}

// 项目根目录（本文件位于 <root>/scripts/ 下），不依赖运行时 cwd
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const modelsDir = path.join(rootDir, 'resources', 'ocr-models');

// 需要下载的模型（来自 ModelScope: RapidAI/RapidOCR）
const MODEL_FILES: ModelFile[] = [
  {
    url: 'https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/master/onnx/PP-OCRv6/rec/PP-OCRv6_rec_tiny.onnx',
    name: 'PP-OCRv6_rec_tiny.onnx',
  },
  {
    url: 'https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/master/paddle/PP-OCRv6/rec/PP-OCRv6_rec_tiny/ppocrv6_tiny_dict.txt',
    name: 'ppocrv6_tiny_dict.txt',
  },
];

const force = process.argv.includes('--force');

async function downloadModel(file: ModelFile): Promise<void> {
  const destPath = path.join(modelsDir, file.name);
  if (existsSync(destPath) && !force) {
    console.log(`[downloadModels] 已存在，跳过: ${file.name}`);
    return;
  }

  console.log(`[downloadModels] 下载中: ${file.name}`);
  const response = await fetch(file.url);
  if (!response.ok || !response.body) {
    throw new Error(`下载失败 (HTTP ${response.status}): ${file.url}`);
  }

  // 先写入临时文件，完成后重命名，避免中断留下半个损坏文件
  const tmpPath = `${destPath}.part`;
  await pipeline(response.body, createWriteStream(tmpPath));
  renameSync(tmpPath, destPath);
  console.log(`[downloadModels] 完成: ${file.name}`);
}

async function main() {
  mkdirSync(modelsDir, { recursive: true });
  for (const file of MODEL_FILES) {
    await downloadModel(file);
  }
  console.log('[downloadModels] 全部完成');
}

main().catch((err) => {
  console.error('[downloadModels] 下载失败:', err);
  process.exit(1);
});
