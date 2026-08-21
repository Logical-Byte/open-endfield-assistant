/**
 * 版本号提升脚本：更新 `src-tauri/tauri.conf.json` 的 `version`，暂存提交并打 tag。
 *
 * `src-tauri/tauri.conf.json` 是版本号的唯一来源：
 * 前端编译期注入（`vite.config.ts`）、打包命名（`scripts/package.ts`）、
 * release tag 校验（`.github/workflows/release.yml`）均以此文件为准，
 * 无需维护 `package.json` / `Cargo.toml` 的版本号。
 *
 * 提交信息遵循 Conventional Commits（`chore: release vX.Y.Z`），tag 为 `vX.Y.Z`；
 * 推送 `v*` tag 会触发 release workflow 自动构建发布（见 `.github/workflows/release.yml`）。
 *
 * 用法:
 *   pnpm bump:version 0.2.0   # 直接指定新版本
 *   pnpm bump:version         # 交互式输入新版本
 */
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath } from 'node:url';
import { valid } from 'semver';

// 项目根目录（本文件位于 <root>/scripts/ 下），不依赖运行时 cwd
const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// 版本号的唯一来源
const configPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json');

interface TauriConfig {
  version?: string;
}

// 读取当前版本号
function readVersion(): string {
  const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
  if (!config.version) {
    throw new Error('`src-tauri/tauri.conf.json` 缺少 version 字段');
  }
  return config.version;
}

// 更新版本号（2 空格缩进写回，补尾部换行，兼容 prettier）
function writeVersion(version: string): void {
  const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
  config.version = version;
  writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`, 'utf8');
}

// 规范化版本号：去除 `v` 前缀并按 semver 严格校验（如 `0.2.0`）
function normalizeVersion(raw: string): string | null {
  const stripped = raw.trim().replace(/^v/i, '');
  return valid(stripped) === stripped ? stripped : null;
}

// 执行 git 命令；失败时抛出带 stderr 的错误
function runGit(args: string[]): string {
  try {
    return execFileSync('git', args, { encoding: 'utf8' });
  } catch (err) {
    const e = err as { stderr?: string; message?: string };
    const detail = (e.stderr ?? e.message ?? '').trim();
    throw new Error(`git ${args.join(' ')} 失败: ${detail || String(err)}`);
  }
}

// 询问一行输入（交互式）
function askLine(prompt: string): Promise<string> {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return rl.question(prompt).finally(() => rl.close());
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const positional = args.filter((arg) => !arg.startsWith('-'));

  const current = readVersion();

  // 新版本：命令行位置参数优先，否则交互式输入
  let input: string;
  if (positional[0]) {
    input = positional[0];
  } else if (process.stdin.isTTY) {
    input = await askLine(`当前版本 ${current}，请输入新版本: `);
  } else {
    console.error('[bump] 未指定版本号，非交互环境下请用参数: pnpm bump:version 0.2.0');
    process.exit(1);
  }

  const next = normalizeVersion(input);
  if (!next) {
    console.error(`[bump] 无效版本号: ${input}（应为干净 semver，如 0.2.0，不接受 v 前缀）`);
    process.exit(1);
  }
  if (next === current) {
    console.error(`[bump] 新版本与当前版本相同: ${current}`);
    process.exit(1);
  }

  const tag = `v${next}`;

  // 打 tag 前先确认不存在，避免留下已提交但打不上 tag 的孤儿提交
  const existing = runGit(['tag', '-l', tag]).trim();
  if (existing === tag) {
    console.error(`[bump] tag 已存在: ${tag}`);
    process.exit(1);
  }

  // 1) 更新版本号（唯一来源）
  writeVersion(next);
  console.log(`[bump] ${current} -> ${next}（${configPath}）`);

  // 2) 暂存并提交（只提交版本文件，不影响其他已暂存/未暂存改动）
  runGit(['add', '--', 'src-tauri/tauri.conf.json']);
  runGit(['commit', '-m', `chore: release ${tag}`, '--', 'src-tauri/tauri.conf.json']);

  // 3) 打 tag
  runGit(['tag', tag]);
  console.log(`[bump] 已提交并打 tag: ${tag}`);
  console.log(`[bump] 推送 tag 触发发布: git push origin ${tag}`);
}

main().catch((err) => {
  console.error(`[bump] 失败: ${err instanceof Error ? err.message : err}`);
  process.exit(1);
});
