import semver from 'semver';

export const R2_BUCKET = 'opendfieldmap-package';
export const PACKAGE_APP_ID = 'oea';
export const PACKAGE_CDN_ORIGIN = 'https://package.oem.re';
export const STABLE_MANIFEST_KEY = `channels/${PACKAGE_APP_ID}/stable.json`;
export const MAX_CACHEABLE_PACKAGE_SIZE = 512 * 1024 * 1024;

const SHA256_PATTERN = /^[0-9a-f]{64}$/;
const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

export interface StableManifest {
  schemaVersion: 1;
  channel: 'stable';
  version: string;
  tag: string;
  filename: string;
  key: string;
  url: string;
  size: number;
  sha256: string;
  publishedAt: string;
}

export type StableUpdateDecision = 'write' | 'skip-same' | 'skip-newer-exists';

export function parseReleaseTag(tag: string): string {
  if (!tag.startsWith('v')) {
    throw new Error(`发布 tag 必须以 v 开头: ${tag}`);
  }

  const version = tag.slice(1);
  if (!semver.valid(version) || semver.clean(version) !== version) {
    throw new Error(`发布 tag 不是规范 SemVer: ${tag}`);
  }

  return version;
}

export function packageFilename(version: string): string {
  return `OEA-windows-x86_64-v${version}.zip`;
}

export function packageObjectKey(tag: string, filename: string): string {
  return `releases/${PACKAGE_APP_ID}/${tag}/${filename}`;
}

export function assertCacheablePackageSize(size: number): void {
  if (!Number.isSafeInteger(size) || size <= 0) {
    throw new Error(`安装包大小无效: ${size}`);
  }
  if (size >= MAX_CACHEABLE_PACKAGE_SIZE) {
    throw new Error(`安装包大小 ${size} 字节达到 Cloudflare 512 MiB 缓存上限，拒绝发布到 stable`);
  }
}

export function parseChecksumSidecar(contents: string, expectedFilename: string): string {
  const match = contents.trim().match(/^([0-9a-fA-F]{64}) {2}([^\r\n]+)$/);
  if (!match) {
    throw new Error('SHA-256 sidecar 格式无效');
  }
  if (match[2] !== expectedFilename) {
    throw new Error(`SHA-256 sidecar 文件名不匹配: ${match[2]}`);
  }
  return match[1].toLowerCase();
}

export function createStableManifest(input: {
  version: string;
  tag: string;
  filename: string;
  size: number;
  sha256: string;
  publishedAt: string;
}): StableManifest {
  const expectedVersion = parseReleaseTag(input.tag);
  if (input.version !== expectedVersion) {
    throw new Error(`版本 ${input.version} 与 tag ${input.tag} 不一致`);
  }
  if (input.filename !== packageFilename(input.version)) {
    throw new Error(`安装包文件名不符合约定: ${input.filename}`);
  }
  assertCacheablePackageSize(input.size);
  if (!SHA256_PATTERN.test(input.sha256)) {
    throw new Error('SHA-256 必须是 64 位小写十六进制');
  }
  if (!ISO_DATE_PATTERN.test(input.publishedAt) || Number.isNaN(Date.parse(input.publishedAt))) {
    throw new Error(`publishedAt 不是规范 UTC ISO-8601 时间: ${input.publishedAt}`);
  }

  const key = packageObjectKey(input.tag, input.filename);
  return {
    schemaVersion: 1,
    channel: 'stable',
    version: input.version,
    tag: input.tag,
    filename: input.filename,
    key,
    url: `${PACKAGE_CDN_ORIGIN}/${key}`,
    size: input.size,
    sha256: input.sha256,
    publishedAt: input.publishedAt,
  };
}

export function validateStableManifest(value: unknown): StableManifest {
  if (typeof value !== 'object' || value === null) {
    throw new Error('stable manifest 必须是对象');
  }

  const manifest = value as Partial<StableManifest>;
  const expectedKeys = [
    'channel',
    'filename',
    'key',
    'publishedAt',
    'schemaVersion',
    'sha256',
    'size',
    'tag',
    'url',
    'version',
  ];
  const actualKeys = Object.keys(value).sort();
  if (
    actualKeys.length !== expectedKeys.length ||
    actualKeys.some((key, index) => key !== expectedKeys[index])
  ) {
    throw new Error('stable manifest 包含未知字段');
  }
  if (manifest.schemaVersion !== 1 || manifest.channel !== 'stable') {
    throw new Error('stable manifest schema/channel 无效');
  }
  if (
    typeof manifest.version !== 'string' ||
    typeof manifest.tag !== 'string' ||
    typeof manifest.filename !== 'string' ||
    typeof manifest.key !== 'string' ||
    typeof manifest.url !== 'string' ||
    typeof manifest.size !== 'number' ||
    typeof manifest.sha256 !== 'string' ||
    typeof manifest.publishedAt !== 'string'
  ) {
    throw new Error('stable manifest 字段缺失或类型无效');
  }

  const canonical = createStableManifest({
    version: manifest.version,
    tag: manifest.tag,
    filename: manifest.filename,
    size: manifest.size,
    sha256: manifest.sha256,
    publishedAt: manifest.publishedAt,
  });
  if (manifest.key !== canonical.key || manifest.url !== canonical.url) {
    throw new Error('stable manifest key/url 不符合固定 CDN 契约');
  }

  return canonical;
}

export function decideStableUpdate(
  current: StableManifest | null,
  candidate: StableManifest,
): StableUpdateDecision {
  if (!current) return 'write';

  const comparison = semver.compare(candidate.version, current.version);
  if (comparison < 0) return 'skip-newer-exists';
  if (comparison > 0) return 'write';

  if (candidate.key !== current.key || candidate.sha256 !== current.sha256) {
    throw new Error(`stable 中的 ${candidate.tag} 与待发布对象哈希或路径不同`);
  }
  return 'skip-same';
}
