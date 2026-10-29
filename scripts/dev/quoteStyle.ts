/**
 * pnpm quote:check：检查自然语言文本中的引号与空格。
 * pnpm quote:fix：删除引号内侧的多余空格、按规则补齐或删除外侧空格后写回文件。
 * 规则见 docs/rule-frontend.md 的「引号与空格」。检查范围为 docs、src、src-tauri、scripts
 * 以及仓库根目录的 Markdown。代码块、行内代码、`resources/` 下的游戏数据、逐字记录游戏原文的
 * 文档和含 ❌ 的反例行不参与。
 * 发现问题时输出位置与原因并以非零状态退出。
 */
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const write = process.argv.includes('--fix');
const directories = ['docs', 'src', 'src-tauri', 'scripts'];
const extensions = new Set(['.md', '.vue', '.ts', '.json', '.rs']);
const skippedDirectories = new Set([
  'node_modules',
  'target',
  'generated',
  'dist',
  'gen',
  'capabilities',
]);
// 逐字记录游戏原文的文档，表格内容必须与游戏一致，不能插入排版空格。
const skippedFiles = new Set(['docs/game-data.md']);
const maskedExtensions = new Set(['.md', '.vue', '.ts', '.rs']);
const word = /[\p{Script=Han}0-9A-Za-z]/u;
const chinesePunctuation = /[\u3000-\u303f\uff01-\uff65]/u;
const exampleMarks = ['❌'];

interface Span {
  text: string;
  isCode: boolean;
  offset: number;
}

interface Problem {
  column: number;
  message: string;
}

let failureCount = 0;

function report(file: string, line: number, problems: Problem[]): void {
  for (const problem of problems) {
    console.error(`${file}:${line}:${problem.column}: ${problem.message}`);
    failureCount++;
  }
}

async function collect(directory: string, files: string[]): Promise<void> {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (!skippedDirectories.has(entry.name)) await collect(path, files);
    } else if (extensions.has(extname(entry.name))) {
      files.push(path);
    }
  }
}

/** 找到与起始位置等长的反引号串，用于界定行内代码。 */
function findClosingTicks(line: string, from: number, length: number): number {
  let index = from;
  while (index < line.length) {
    if (line[index] !== '`') {
      index++;
      continue;
    }
    let end = index;
    while (end < line.length && line[end] === '`') end++;
    if (end - index === length) return index;
    index = end;
  }
  return -1;
}

function splitSpans(line: string, maskCode: boolean): Span[] {
  if (!maskCode) return [{ text: line, isCode: false, offset: 0 }];
  const spans: Span[] = [];
  let plainStart = 0;
  let index = 0;
  while (index < line.length) {
    if (line[index] !== '`') {
      index++;
      continue;
    }
    let runEnd = index;
    while (runEnd < line.length && line[runEnd] === '`') runEnd++;
    const closing = findClosingTicks(line, runEnd, runEnd - index);
    if (closing === -1) {
      index = runEnd;
      continue;
    }
    if (plainStart < index)
      spans.push({ text: line.slice(plainStart, index), isCode: false, offset: plainStart });
    spans.push({ text: line.slice(index, runEnd), isCode: true, offset: index });
    spans.push({ text: line.slice(runEnd, closing), isCode: true, offset: runEnd });
    index = closing;
    let ticksEnd = closing;
    while (ticksEnd < line.length && line[ticksEnd] === '`') ticksEnd++;
    spans.push({ text: line.slice(closing, ticksEnd), isCode: true, offset: closing });
    index = ticksEnd;
    plainStart = index;
  }
  if (plainStart < line.length)
    spans.push({ text: line.slice(plainStart), isCode: false, offset: plainStart });
  return spans;
}

function isWord(character: string | undefined): boolean {
  return character !== undefined && word.test(character);
}

function isChinesePunctuation(character: string | undefined): boolean {
  return character !== undefined && chinesePunctuation.test(character);
}

/** 含 ✅/❌ 的行是规则文档里的示例，可能故意不规范。 */
function isExample(line: string): boolean {
  return exampleMarks.some((mark) => line.includes(mark));
}

function markFences(lines: string[], extension: string): boolean[] {
  let fence = false;
  return lines.map((line) => {
    if (extension !== '.md') return false;
    const trimmed = line.trimStart();
    if (trimmed.startsWith('```') || trimmed.startsWith('~~~')) {
      fence = !fence;
      return true;
    }
    return fence;
  });
}

function fixLine(line: string, maskCode: boolean): string {
  return splitSpans(line, maskCode)
    .map((span) => (span.isCode ? span.text : processProse(span, false, true).text))
    .join('');
}

function inspectProse(span: Span, checkStraightQuotes: boolean): Problem[] {
  return processProse(span, checkStraightQuotes, false).problems;
}

function processProse(
  span: Span,
  checkStraightQuotes: boolean,
  repair: boolean,
): { text: string; problems: Problem[] } {
  const { text, offset } = span;
  const output: string[] = [];
  const problems: Problem[] = [];
  const problem = (index: number, message: string) => {
    if (!repair) problems.push({ column: offset + index + 1, message });
  };

  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    if (character === '“' || character === '‘') {
      const previous = text[index - 1];
      if (text[index + 1] === ' ') {
        problem(index, '引号内侧不应有空格');
        output.push(character);
        index++;
        continue;
      }
      if (isWord(previous)) {
        problem(index, '引号外侧缺少空格');
        output.push(' ');
      } else if (previous === ' ' && isChinesePunctuation(text[index - 2])) {
        problem(index - 1, '标点与引号之间不应有空格');
        output.pop();
      }
      output.push(character);
      continue;
    }
    if (character === '”' || character === '’') {
      if (text[index - 1] === ' ') {
        problem(index, '引号内侧不应有空格');
        output.pop();
      }
      output.push(character);
      if (isWord(text[index + 1])) {
        problem(index, '引号外侧缺少空格');
        output.push(' ');
      } else if (text[index + 1] === ' ' && isChinesePunctuation(text[index + 2])) {
        problem(index + 1, '标点与引号之间不应有空格');
        index++;
      }
      continue;
    }
    if (character === '"' && checkStraightQuotes) problem(index, '自然语言使用弯引号');
    output.push(character);
  }
  return { text: output.join(''), problems };
}

async function main(): Promise<void> {
  const files: string[] = [];
  for (const entry of await readdir(root, { withFileTypes: true }))
    if (entry.isFile() && entry.name.endsWith('.md')) files.push(join(root, entry.name));
  for (const directory of directories) await collect(join(root, directory), files);

  for (const path of files) {
    const name = relative(root, path).split(sep).join('/');
    if (skippedFiles.has(name)) continue;
    const extension = extname(path);
    const maskCode = maskedExtensions.has(extension);
    const original = await readFile(path, 'utf8');
    const lines = original.split('\n');
    const fenced = markFences(lines, extension);
    const source = write
      ? lines.map((line, index) =>
          fenced[index] || isExample(line) ? line : fixLine(line, maskCode),
        )
      : lines;
    if (write) {
      const updated = source.join('\n');
      if (updated !== original) await writeFile(path, updated);
    }
    for (const [index, line] of source.entries()) {
      if (fenced[index] || isExample(line)) continue;
      for (const span of splitSpans(line, maskCode)) {
        if (span.isCode) continue;
        report(name, index + 1, inspectProse(span, extension === '.md'));
      }
    }
  }
  if (failureCount) process.exitCode = 1;
}

await main();
