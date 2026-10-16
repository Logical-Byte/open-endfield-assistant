import { describe, expect, it } from 'vitest';
import {
  MAX_CACHEABLE_PACKAGE_SIZE,
  createStableManifest,
  decideStableUpdate,
  packageFilename,
  parseChecksumSidecar,
  parseReleaseTag,
  validateStableManifest,
} from './publishR2Core';

const SHA_A = 'a'.repeat(64);
const SHA_B = 'b'.repeat(64);

function manifest(version: string, sha256 = SHA_A) {
  return createStableManifest({
    version,
    tag: `v${version}`,
    filename: packageFilename(version),
    size: 1024,
    sha256,
    publishedAt: '2026-08-30T12:00:00.000Z',
  });
}

describe('R2 release contract', () => {
  it('accepts canonical release tags and checksum sidecars', () => {
    expect(parseReleaseTag('v1.2.3')).toBe('1.2.3');
    expect(
      parseChecksumSidecar(`${SHA_A}  OEA-windows-x86_64-v1.2.3.zip\n`, packageFilename('1.2.3')),
    ).toBe(SHA_A);
  });

  it('rejects malformed tags, filenames and checksum sidecars', () => {
    expect(() => parseReleaseTag('1.2.3')).toThrow();
    expect(() => parseReleaseTag('v01.2.3')).toThrow();
    expect(() => parseChecksumSidecar(`${SHA_A}  wrong.zip`, packageFilename('1.2.3'))).toThrow();
  });

  it('uses an immutable versioned CDN URL', () => {
    expect(manifest('1.2.3')).toMatchObject({
      key: 'releases/oea/v1.2.3/OEA-windows-x86_64-v1.2.3.zip',
      url: 'https://package.oem.re/releases/oea/v1.2.3/OEA-windows-x86_64-v1.2.3.zip',
    });
  });

  it('rejects packages at the 512 MiB cache limit', () => {
    expect(() =>
      createStableManifest({
        version: '1.2.3',
        tag: 'v1.2.3',
        filename: packageFilename('1.2.3'),
        size: MAX_CACHEABLE_PACKAGE_SIZE,
        sha256: SHA_A,
        publishedAt: '2026-08-30T12:00:00.000Z',
      }),
    ).toThrow(/512 MiB/);
  });

  it('never moves stable backwards during ordinary publishing', () => {
    expect(decideStableUpdate(manifest('1.2.3'), manifest('1.2.2'))).toBe('skip-newer-exists');
    expect(decideStableUpdate(manifest('1.2.3'), manifest('1.2.4'))).toBe('write');
    expect(decideStableUpdate(manifest('1.2.3'), manifest('1.2.3'))).toBe('skip-same');
  });

  it('rejects a different object for an already-stable version', () => {
    expect(() => decideStableUpdate(manifest('1.2.3'), manifest('1.2.3', SHA_B))).toThrow();
  });

  it('rejects manifests that can redirect outside the fixed CDN contract', () => {
    expect(() =>
      validateStableManifest({ ...manifest('1.2.3'), url: 'https://example.com/payload.zip' }),
    ).toThrow();
  });
});
