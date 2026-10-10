import { readFile, writeFile } from 'node:fs/promises';
import { baseCompile } from '@intlify/message-compiler';

const locales = ['zh-CN', 'en'] as const;
const dictionaries: Record<string, Record<string, string>> = {};
let failed = false;
function error(message: string): void {
  console.error(message);
  failed = true;
}
for (const locale of locales) {
  const path = new URL(`../src/shared/i18n/locales/${locale}.json`, import.meta.url);
  const dictionary: Record<string, string> = JSON.parse(await readFile(path, 'utf8'));
  dictionaries[locale] = dictionary;
  const keys = Object.keys(dictionary);
  const sorted = [...keys].sort();
  if (process.argv.includes('--fix')) {
    await writeFile(
      path,
      `${JSON.stringify(Object.fromEntries(sorted.map((key) => [key, dictionary[key]])), null, 2)}\n`,
    );
  } else if (keys.some((key, index) => key !== sorted[index]))
    error(`${locale}: keys must be sorted`);
}
const reference = Object.keys(dictionaries['zh-CN']).sort();
const parameters: Record<string, Record<string, string[]>> = {};
for (const locale of locales) {
  const dictionary = dictionaries[locale];
  if (JSON.stringify(Object.keys(dictionary).sort()) !== JSON.stringify(reference))
    error(`${locale}: translation keys differ`);
  parameters[locale] = {};
  for (const [key, message] of Object.entries(dictionary)) {
    if (typeof message !== 'string' || !message.trim()) {
      error(`${locale} ${key}: empty message`);
      continue;
    }
    if (/<\/?[a-z][^>]*>/i.test(message)) error(`${locale} ${key}: HTML is not allowed`);
    const { ast } = baseCompile(message, {
      onError(issue) {
        error(`${locale} ${key}: ${issue.message}`);
      },
    });
    const names = new Set<string>();
    // message-compiler 的 const enum 在运行时被擦除：Named = 4，List = 5。
    function visit(node: unknown): void {
      if (Array.isArray(node)) {
        node.forEach(visit);
        return;
      }
      if (!node || typeof node !== 'object') return;
      const item = node as Record<string, unknown>;
      if (item.type === 4 && typeof item.key === 'string') names.add(item.key);
      if (item.type === 5) error(`${locale} ${key}: use named parameters`);
      Object.values(item).forEach(visit);
    }
    visit(ast);
    parameters[locale][key] = [...names].sort();
  }
}
for (const key of reference) {
  if (JSON.stringify(parameters['zh-CN'][key]) !== JSON.stringify(parameters['en'][key]))
    error(`${key}: named parameters differ`);
}
if (failed) process.exitCode = 1;
