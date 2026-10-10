/**
 * pnpm i18n:check：检查消息的嵌套结构、属性排序、非空文本及插值，禁止 HTML 和管道复数。
 * pnpm i18n:fix：执行相同检查，并在通过后按层级排序属性、写回 JSON。
 * 按 locales 列表顺序检查各语言文件。发现问题时输出语言与消息路径并以非零状态退出。
 */
import { readFile, writeFile } from 'node:fs/promises';

const locales = ['zh-CN', 'en'] as const;
const messages: Record<string, Record<string, string>> = {};
const directory = new URL('../../src/shared/i18n/locales/', import.meta.url);
const fix = process.argv.includes('--fix');
let failureCount = 0;
function error(locale: string, key: string, message: string): void {
  console.error(`${locale} ${key}: ${message}`);
  failureCount++;
}

/** 返回排序后的树，始终保留源文件的嵌套结构。 */
function check(locale: string, value: unknown, key: string): unknown {
  if (typeof value === 'string') {
    messages[locale][key] = value;
    if (!value.trim()) error(locale, key, 'empty message');
    if (/<\/?[a-z][^>]*>/i.test(value)) error(locale, key, 'HTML is not allowed');
    // {{name}} 是数值插值，{slot} 是 i18next-vue 的组件插值。
    const literal = value
      .replace(/\{\{\s*[a-zA-Z_]\w*\s*\}\}/g, '')
      .replace(/\{\s*[a-zA-Z0-9-]+\s*\}/g, '');
    if (/[{}]/.test(literal))
      error(locale, key, 'use named interpolation {{name}} or component slots {slot}');
    if (value.includes('|')) error(locale, key, 'use i18next plural keys instead of pipe plurals');
    return value;
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    error(locale, key, 'expected a message string or nested object');
    return value;
  }
  const dictionary = value as Record<string, unknown>;
  const keys = Object.keys(dictionary);
  const sorted = [...keys].sort();
  if (!fix && keys.some((name, index) => name !== sorted[index]))
    error(locale, key, 'keys must be sorted');
  return Object.fromEntries(
    sorted.map((name) => {
      const childKey = key ? `${key}.${name}` : name;
      if (!name || /[.:]/.test(name))
        error(locale, childKey, 'property names must not contain path separators');
      return [name, check(locale, dictionary[name], childKey)];
    }),
  );
}

for (const locale of locales) {
  const path = new URL(`${locale}.json`, directory);
  const dictionary: unknown = JSON.parse(await readFile(path, 'utf8'));
  messages[locale] = {};
  const previousFailures = failureCount;
  const sorted = check(locale, dictionary, '');
  if (fix && failureCount === previousFailures)
    await writeFile(path, `${JSON.stringify(sorted, null, 2)}\n`);
}
// 各语言的复数类别可以不同，比较基本 key 和每个分支的参数。
function messageKey(key: string): string {
  return key.replace(/_(zero|one|two|few|many|other)$/, '');
}
function parameters(message: string): string[] {
  return [
    ...new Set([...message.matchAll(/\{\{?\s*([\w-]+)\s*\}\}?/g)].map((match) => match[1])),
  ].sort();
}
const reference = Object.fromEntries(
  Object.entries(messages['zh-CN']).map(([key, message]) => [messageKey(key), parameters(message)]),
);
for (const locale of locales.slice(1)) {
  const keys = [...new Set(Object.keys(messages[locale]).map(messageKey))].sort();
  if (JSON.stringify(keys) !== JSON.stringify(Object.keys(reference).sort()))
    error(locale, '', 'translation keys differ');
  for (const [key, message] of Object.entries(messages[locale])) {
    if (JSON.stringify(parameters(message)) !== JSON.stringify(reference[messageKey(key)]))
      error(locale, key, 'named parameters differ');
  }
}
if (failureCount) process.exitCode = 1;
