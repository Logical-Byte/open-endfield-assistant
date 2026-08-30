import {
  GetObjectCommand,
  HeadObjectCommand,
  PutObjectCommand,
  S3Client,
  type HeadObjectCommandOutput,
} from '@aws-sdk/client-s3';
import { Upload } from '@aws-sdk/lib-storage';
import { createHash } from 'node:crypto';
import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { pipeline } from 'node:stream/promises';
import { Writable } from 'node:stream';
import {
  R2_BUCKET,
  STABLE_MANIFEST_KEY,
  createStableManifest,
  decideStableUpdate,
  packageFilename,
  packageObjectKey,
  parseChecksumSidecar,
  parseReleaseTag,
  validateStableManifest,
  type StableManifest,
} from './publishR2Core';

type Operation = 'publish' | 'promote';
type Metadata = Record<string, string>;

interface CliOptions {
  operation: Operation;
  tag: string;
  artifactDir: string;
}

interface ImmutableObjectInput {
  localPath: string;
  key: string;
  contentType: string;
  contentDisposition: string;
  cacheControl: string;
  expectedSize: number;
  expectedMetadata: Metadata;
  metadata?: Metadata;
}

function parseArgs(argv: string[]): CliOptions {
  const operation = argv[0];
  if (operation !== 'publish' && operation !== 'promote') {
    throw new Error('用法: publishR2.ts <publish|promote> --tag vX.Y.Z [--artifact-dir DIR]');
  }

  function readOption(name: string): string | undefined {
    const index = argv.indexOf(name);
    return index >= 0 ? argv[index + 1] : undefined;
  }

  const tag = readOption('--tag');
  if (!tag) throw new Error('缺少 --tag');

  return {
    operation,
    tag,
    artifactDir: path.resolve(readOption('--artifact-dir') ?? 'releases'),
  };
}

function requiredEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`缺少环境变量 ${name}`);
  return value;
}

function createR2Client(): S3Client {
  const accountId = requiredEnv('R2_ACCOUNT_ID');
  return new S3Client({
    region: 'auto',
    endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
    credentials: {
      accessKeyId: requiredEnv('R2_ACCESS_KEY_ID'),
      secretAccessKey: requiredEnv('R2_SECRET_ACCESS_KEY'),
    },
  });
}

function isNotFound(error: unknown): boolean {
  if (typeof error !== 'object' || error === null) return false;
  const candidate = error as { name?: string; $metadata?: { httpStatusCode?: number } };
  return candidate.name === 'NotFound' || candidate.$metadata?.httpStatusCode === 404;
}

async function headObject(client: S3Client, key: string): Promise<HeadObjectCommandOutput | null> {
  try {
    return await client.send(new HeadObjectCommand({ Bucket: R2_BUCKET, Key: key }));
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}

function assertMetadata(actual: Metadata | undefined, expected: Metadata, key: string): void {
  for (const [name, value] of Object.entries(expected)) {
    if (actual?.[name.toLowerCase()] !== value) {
      throw new Error(`R2 对象 ${key} 的 metadata.${name} 不匹配`);
    }
  }
}

function verifyHead(
  head: HeadObjectCommandOutput,
  key: string,
  expectedSize: number,
  expectedMetadata: Metadata,
): void {
  if (head.ContentLength !== expectedSize) {
    throw new Error(`R2 对象 ${key} 大小不匹配`);
  }
  assertMetadata(head.Metadata, expectedMetadata, key);
}

async function uploadImmutableObject(
  client: S3Client,
  input: ImmutableObjectInput,
): Promise<HeadObjectCommandOutput> {
  const existing = await headObject(client, input.key);
  if (existing) {
    verifyHead(existing, input.key, input.expectedSize, input.expectedMetadata);
    console.log(`[r2] 对象已存在且校验一致，跳过上传: ${input.key}`);
    return existing;
  }

  console.log(`[r2] 上传不可变对象: ${input.key}`);
  const upload = new Upload({
    client,
    leavePartsOnError: false,
    queueSize: 4,
    partSize: 16 * 1024 * 1024,
    params: {
      Bucket: R2_BUCKET,
      Key: input.key,
      Body: createReadStream(input.localPath),
      ContentType: input.contentType,
      ContentDisposition: input.contentDisposition,
      CacheControl: input.cacheControl,
      Metadata: input.metadata ?? input.expectedMetadata,
    },
  });
  await upload.done();

  const uploaded = await headObject(client, input.key);
  if (!uploaded) throw new Error(`上传完成后无法读取 R2 对象: ${input.key}`);
  verifyHead(uploaded, input.key, input.expectedSize, input.expectedMetadata);
  return uploaded;
}

async function sha256File(filePath: string): Promise<string> {
  const hash = createHash('sha256');
  await pipeline(
    createReadStream(filePath),
    new Writable({
      write(chunk: Buffer, _encoding, callback): void {
        hash.update(chunk);
        callback();
      },
    }),
  );
  return hash.digest('hex');
}

async function sha256Object(client: S3Client, key: string): Promise<string> {
  const response = await client.send(new GetObjectCommand({ Bucket: R2_BUCKET, Key: key }));
  if (!response.Body) throw new Error(`R2 对象响应体为空: ${key}`);
  const hash = createHash('sha256');
  for await (const chunk of response.Body as AsyncIterable<Uint8Array>) {
    hash.update(chunk);
  }
  return hash.digest('hex');
}

async function readStableManifest(client: S3Client): Promise<StableManifest | null> {
  try {
    const response = await client.send(
      new GetObjectCommand({ Bucket: R2_BUCKET, Key: STABLE_MANIFEST_KEY }),
    );
    if (!response.Body) throw new Error('stable manifest 响应体为空');
    return validateStableManifest(JSON.parse(await response.Body.transformToString()));
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}

async function writeStableManifest(client: S3Client, manifest: StableManifest): Promise<void> {
  await client.send(
    new PutObjectCommand({
      Bucket: R2_BUCKET,
      Key: STABLE_MANIFEST_KEY,
      Body: `${JSON.stringify(manifest, null, 2)}\n`,
      ContentType: 'application/json; charset=utf-8',
      CacheControl: 'no-store',
      Metadata: {
        channel: 'stable',
        version: manifest.version,
        tag: manifest.tag,
        sha256: manifest.sha256,
      },
    }),
  );
  console.log(`[r2] stable 已切换到 ${manifest.tag}`);
}

async function publish(client: S3Client, options: CliOptions): Promise<void> {
  const version = parseReleaseTag(options.tag);
  const filename = packageFilename(version);
  const zipPath = path.join(options.artifactDir, filename);
  const checksumPath = `${zipPath}.sha256`;
  if (!existsSync(zipPath) || !existsSync(checksumPath)) {
    throw new Error(`缺少发布文件: ${zipPath} 或 ${checksumPath}`);
  }

  const size = statSync(zipPath).size;
  const sidecar = readFileSync(checksumPath, 'utf8');
  const expectedSha256 = parseChecksumSidecar(sidecar, filename);
  const actualSha256 = await sha256File(zipPath);
  if (actualSha256 !== expectedSha256) {
    throw new Error('本地安装包 SHA-256 与 sidecar 不一致');
  }

  const current = await readStableManifest(client);
  const candidate = createStableManifest({
    version,
    tag: options.tag,
    filename,
    size,
    sha256: actualSha256,
    publishedAt: new Date().toISOString(),
  });
  if (
    current?.version === candidate.version &&
    (current.key !== candidate.key || current.sha256 !== candidate.sha256)
  ) {
    throw new Error(`stable 中的 ${options.tag} 与待发布对象不一致，拒绝覆盖`);
  }

  const key = packageObjectKey(options.tag, filename);
  const zipHead = await uploadImmutableObject(client, {
    localPath: zipPath,
    key,
    contentType: 'application/zip',
    contentDisposition: `attachment; filename="${filename}"`,
    cacheControl: 'public, max-age=31536000, immutable',
    expectedSize: size,
    expectedMetadata: {
      sha256: actualSha256,
      tag: options.tag,
      version,
    },
    metadata: {
      sha256: actualSha256,
      tag: options.tag,
      version,
      'published-at': candidate.publishedAt,
    },
  });

  const checksumFilename = `${filename}.sha256`;
  await uploadImmutableObject(client, {
    localPath: checksumPath,
    key: `${key}.sha256`,
    contentType: 'text/plain; charset=utf-8',
    contentDisposition: `attachment; filename="${checksumFilename}"`,
    cacheControl: 'public, max-age=31536000, immutable',
    expectedSize: statSync(checksumPath).size,
    expectedMetadata: {
      sha256: actualSha256,
      tag: options.tag,
      version,
      artifact: 'checksum',
    },
  });

  const objectPublishedAt = zipHead.Metadata?.['published-at'];
  if (!objectPublishedAt) throw new Error(`R2 对象 ${key} 缺少 published-at metadata`);
  const verifiedCandidate = createStableManifest({ ...candidate, publishedAt: objectPublishedAt });
  const decision = decideStableUpdate(current, verifiedCandidate);
  if (decision === 'write') {
    await writeStableManifest(client, verifiedCandidate);
  } else {
    console.log(
      decision === 'skip-same'
        ? `[r2] ${options.tag} 已是 stable，无需改写`
        : `[r2] stable 已是更高版本 ${current?.tag}，仅保留 ${options.tag} 归档`,
    );
  }
}

async function promote(client: S3Client, options: CliOptions): Promise<void> {
  const version = parseReleaseTag(options.tag);
  const filename = packageFilename(version);
  const key = packageObjectKey(options.tag, filename);
  const head = await headObject(client, key);
  if (!head?.ContentLength) throw new Error(`待提升的 R2 对象不存在: ${key}`);

  const checksumResponse = await client.send(
    new GetObjectCommand({ Bucket: R2_BUCKET, Key: `${key}.sha256` }),
  );
  if (!checksumResponse.Body) throw new Error(`checksum 对象响应体为空: ${key}.sha256`);
  const sidecarSha256 = parseChecksumSidecar(
    await checksumResponse.Body.transformToString(),
    filename,
  );
  const metadataSha256 = head.Metadata?.sha256;
  if (!metadataSha256 && (await sha256Object(client, key)) !== sidecarSha256) {
    throw new Error(`R2 对象内容与 checksum 不一致: ${key}`);
  }
  if (metadataSha256 && metadataSha256 !== sidecarSha256) {
    throw new Error(`checksum 与对象 metadata 不一致: ${key}`);
  }
  if (head.Metadata?.tag && head.Metadata.tag !== options.tag) {
    throw new Error(`R2 对象 tag metadata 不匹配: ${key}`);
  }
  if (head.Metadata?.version && head.Metadata.version !== version) {
    throw new Error(`R2 对象 version metadata 不匹配: ${key}`);
  }
  const publishedAt = head.Metadata?.['published-at'] ?? head.LastModified?.toISOString();
  if (!publishedAt) throw new Error(`待提升的 R2 对象缺少可用发布时间: ${key}`);

  const manifest = createStableManifest({
    version,
    tag: options.tag,
    filename,
    size: head.ContentLength,
    sha256: sidecarSha256,
    publishedAt,
  });
  await writeStableManifest(client, manifest);
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2));
  const client = createR2Client();
  if (options.operation === 'publish') await publish(client, options);
  else await promote(client, options);
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[r2] 发布失败: ${message}`);
  process.exit(1);
});
