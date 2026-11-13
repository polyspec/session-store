// The install of the pinned release of bay (SE-10.4, scripts/install-bay.mts): the archive of the platform is the one
// that config/toolchain.json names, an archive of another sha256 installs nothing, and bay and bayd are put into the
// directory of the install. The test gives the install an archive that it packs itself, so it downloads nothing.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test, type TestContext } from 'node:test';

import { archiveOf, install, type Pin } from './install-bay.mts';

/** A directory that the test removes. */
function temporary(t: TestContext): string {
  const directory = mkdtempSync(join(tmpdir(), 'session-store-install-bay-'));
  t.after(() => { rmSync(directory, { recursive: true, force: true }); });
  return directory;
}

/** The bytes of an archive of the release `version` for `platform`, as a release packs it: a directory with bay and bayd. */
function packed(t: TestContext, version: string, platform: string): Buffer {
  const directory = temporary(t);
  const name = `bay_${version}_${platform}`;
  mkdirSync(join(directory, name));
  for (const program of ['bay', 'bayd']) {
    writeFileSync(join(directory, name, program), `#!/bin/sh\necho ${program} ${version}\n`);
    chmodSync(join(directory, name, program), 0o755);
  }
  const archive = join(directory, 'archive.tar.gz');
  assert.equal(spawnSync('tar', ['-czf', archive, '-C', directory, name]).status, 0);
  return readFileSync(archive);
}

const sha256 = (bytes: Buffer): string => createHash('sha256').update(bytes).digest('hex');

test('the archive of a platform is the asset of the pinned release that the release names for it', () => {
  const pin: Pin = { version: 'v1.2.3', releases: 'https://example.invalid/bay/releases', sha256: { 'linux-arm64': 'a'.repeat(64), 'linux-x64': 'b'.repeat(64), 'darwin-arm64': 'c'.repeat(64) } };
  assert.deepEqual(archiveOf(pin, 'linux-arm64'), { name: 'bay_v1.2.3_linux_arm64', url: 'https://example.invalid/bay/releases/download/v1.2.3/bay_v1.2.3_linux_arm64.tar.gz', sha256: 'a'.repeat(64) });
  // The release names the architecture x64 amd64.
  assert.deepEqual(archiveOf(pin, 'linux-x64'), { name: 'bay_v1.2.3_linux_amd64', url: 'https://example.invalid/bay/releases/download/v1.2.3/bay_v1.2.3_linux_amd64.tar.gz', sha256: 'b'.repeat(64) });
  assert.equal(archiveOf(pin, 'darwin-arm64').name, 'bay_v1.2.3_darwin_arm64');
  assert.throws(() => archiveOf(pin, 'win32-x64'), /config\/toolchain\.json holds no sha256 of bay for win32-x64/);
});

test('an archive of the pinned sha256 puts bay and bayd into the directory of the install', async (t) => {
  const bytes = packed(t, 'v1.2.3', 'linux_arm64');
  const pin: Pin = { version: 'v1.2.3', releases: 'https://example.invalid/bay/releases', sha256: { 'linux-arm64': sha256(bytes) } };
  const directory = join(temporary(t), 'bay');
  const asked: string[] = [];
  await install(pin, 'linux-arm64', directory, (url) => { asked.push(url); return Promise.resolve(bytes); });
  assert.deepEqual(asked, ['https://example.invalid/bay/releases/download/v1.2.3/bay_v1.2.3_linux_arm64.tar.gz']);
  for (const program of ['bay', 'bayd']) {
    assert.equal(readFileSync(join(directory, program), 'utf8'), `#!/bin/sh\necho ${program} v1.2.3\n`);
    assert.equal(statSync(join(directory, program)).mode & 0o111, 0o111, `${program} is not executable`);
  }
  // A second install replaces the first.
  const next = packed(t, 'v1.2.4', 'linux_arm64');
  await install({ ...pin, version: 'v1.2.4', sha256: { 'linux-arm64': sha256(next) } }, 'linux-arm64', directory, () => Promise.resolve(next));
  assert.equal(readFileSync(join(directory, 'bay'), 'utf8'), '#!/bin/sh\necho bay v1.2.4\n');
});

test('an archive of another sha256 installs nothing and names both digests', async (t) => {
  const bytes = packed(t, 'v1.2.3', 'linux_arm64');
  const pin: Pin = { version: 'v1.2.3', releases: 'https://example.invalid/bay/releases', sha256: { 'linux-arm64': 'a'.repeat(64) } };
  const directory = join(temporary(t), 'bay');
  await assert.rejects(install(pin, 'linux-arm64', directory, () => Promise.resolve(bytes)), new RegExp(`the sha256 of bay_v1\\.2\\.3_linux_arm64\\.tar\\.gz is ${sha256(bytes)}; config/toolchain\\.json pins ${'a'.repeat(64)}`));
  assert.equal(existsSync(directory), false);
});

test('an archive without bay or bayd installs nothing', async (t) => {
  const source = temporary(t);
  mkdirSync(join(source, 'bay_v1.2.3_linux_arm64'));
  writeFileSync(join(source, 'bay_v1.2.3_linux_arm64', 'bay'), '');
  const archive = join(source, 'archive.tar.gz');
  assert.equal(spawnSync('tar', ['-czf', archive, '-C', source, 'bay_v1.2.3_linux_arm64']).status, 0);
  const bytes = readFileSync(archive);
  const directory = join(temporary(t), 'bay');
  await assert.rejects(install({ version: 'v1.2.3', releases: 'https://example.invalid/bay/releases', sha256: { 'linux-arm64': sha256(bytes) } }, 'linux-arm64', directory, () => Promise.resolve(bytes)), /bay_v1\.2\.3_linux_arm64\.tar\.gz holds no bay_v1\.2\.3_linux_arm64\/bayd/);
  assert.equal(existsSync(directory), false);
});
