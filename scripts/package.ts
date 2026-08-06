/**
 * 打包脚本：将 release 产物组装为绿色便携 zip。
 *
 * 产物结构（zip 根目录）：
 *   OEA.exe
 *   models/
 *   resources/
 *
 * 产物命名：`<productName>-windows-<arch>-v<version>.zip`，例如 `OEA-windows-x86_64-v0.1.0.zip`。
 *
 * 执行方式: `pnpm package`
 */
import {
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { ZipFile } from 'yazl';

interface TauriConfig {
  productName?: string;
  version?: string;
}

// 项目根目录（本文件位于 <root>/scripts/ 下），不依赖运行时 cwd
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// 读取 tauri.conf.json（版本与产品名的唯一事实来源）
const tauriConfig = JSON.parse(
  readFileSync(path.join(rootDir, 'src-tauri', 'tauri.conf.json'), 'utf8'),
) as TauriConfig;
const productName = tauriConfig.productName ?? 'OEA';
const version = tauriConfig.version ?? '0.0.0';

// 目标架构：Node 的 arch 命名 → 产物命名
const ARCH_MAP: Record<string, string> = { x64: 'x86_64', arm64: 'aarch64', ia32: 'i686' };
const arch = ARCH_MAP[process.arch] ?? process.arch;

// 产物与目录
const bundleName = `${productName}-windows-${arch}-v${version}`;
const outDir = path.join(rootDir, 'release');
const zipPath = path.join(outDir, `${bundleName}.zip`);
const stagingDir = path.join(outDir, bundleName);

async function main() {
  // 定位 release 主程序：--no-bundle 构建时二进制沿用 Cargo 包名（如 oea.exe），
  // 而非 productName，需要在这里重命名。
  const exePath = findReleaseExe();
  if (!exePath) {
    console.error('[package] 未找到 release 主程序，请先运行: tauri build --no-bundle');
    process.exit(1);
  }

  // 组装暂存目录
  console.log(`[package] 组装 ${bundleName}`);
  rmSync(stagingDir, { recursive: true, force: true });
  mkdirSync(stagingDir, { recursive: true });

  // 1) 主程序（重命名为 productName）
  copyFileSync(exePath, path.join(stagingDir, `${productName}.exe`));

  // 2) models / resources（跳过 `.` 开头的条目：子模块 .git、.gitignore 等）
  for (const dir of ['models', 'resources']) {
    const src = path.join(rootDir, dir);
    if (!existsSync(src)) {
      console.error(`[package] 缺少 ${dir}/ 目录: ${src}`);
      process.exit(1);
    }
    copyDirFiltered(src, path.join(stagingDir, dir));
  }

  // 打 zip：使用 yazl（自动为中文等非 ASCII 文件名设置 UTF-8 编码标志，
  // 避免 bsdtar 打出的 zip 被资源管理器解压成乱码）
  console.log(`[package] 生成 zip: ${zipPath}`);
  rmSync(zipPath, { force: true });
  await createZip(stagingDir, zipPath);

  // 清理暂存目录
  rmSync(stagingDir, { recursive: true, force: true });

  // 汇总
  const sizeMB = (statSync(zipPath).size / 1024 / 1024).toFixed(1);
  console.log(`[package] 完成: ${zipPath} (${sizeMB} MB)`);
}

// 在 release 目录中查找主程序：优先 productName，其次 Cargo 包名
function findReleaseExe(): string | null {
  const releaseDir = path.join(rootDir, 'src-tauri', 'target', 'release');
  const cargoName = readFileSync(path.join(rootDir, 'src-tauri', 'Cargo.toml'), 'utf8').match(
    /^\s*name\s*=\s*"([^"]+)"/m,
  )?.[1];
  const candidates = [productName, cargoName]
    .filter(Boolean)
    .map((n) => path.join(releaseDir, `${n}.exe`));
  const exePath = candidates.find((p) => existsSync(p));
  if (!exePath) {
    console.error(`  已查找: ${candidates.join(', ')}`);
  }
  return exePath ?? null;
}

// 递归拷贝目录，跳过 `.` 开头的条目（.git、.gitignore 等）
function copyDirFiltered(src: string, dest: string) {
  mkdirSync(dest, { recursive: true });
  for (const entry of readdirSync(src)) {
    if (entry.startsWith('.')) continue;
    const s = path.join(src, entry);
    const d = path.join(dest, entry);
    if (statSync(s).isDirectory()) copyDirFiltered(s, d);
    else copyFileSync(s, d);
  }
}

// 递归列出目录下所有文件的相对路径（zip 条目统一使用 / 分隔符）
function walkFiles(dir: string, prefix = ''): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    if (entry.startsWith('.')) continue;
    const full = path.join(dir, entry);
    const rel = prefix ? `${prefix}/${entry}` : entry;
    if (statSync(full).isDirectory()) files.push(...walkFiles(full, rel));
    else files.push(rel);
  }
  return files;
}

// 用 yazl 将暂存目录打包为 zip
function createZip(dir: string, zipPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const zip = new ZipFile();
    for (const rel of walkFiles(dir)) {
      zip.addFile(path.join(dir, rel), rel);
    }
    const writeStream = createWriteStream(zipPath);
    writeStream.on('close', () => resolve());
    writeStream.on('error', reject);
    zip.outputStream.on('error', reject);
    zip.outputStream.pipe(writeStream);
    zip.end();
  });
}

main().catch((err) => {
  console.error('[package] 打包失败:', err);
  process.exit(1);
});
