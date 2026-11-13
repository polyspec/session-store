// Installs the release of bay that is pinned for CI into var/tools/bay (SE-10.4), for a machine that holds no bay,
// such as a runner of CI: `make install-bay`. No run reads the pin. config/toolchain.json names the version, the address of the releases and the sha256 of
// the archive of each platform. The archive is used only when its sha256 is the pinned one; bay and bayd are then put
// into the directory, each by one rename, so that a failed install leaves the directory as it was. The install
// changes no setup of the machine: `bay install` does, which `make setup-bay` runs.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The pin of bay in config/toolchain.json. */
export interface Pin {
  /** The version of the release, which is its tag. */
  version: string;
  /** The address of the releases of bay. */
  releases: string;
  /** The sha256 of the archive of each platform, by `<platform>-<architecture>` as Node.js names them. */
  sha256: Record<string, string>;
}

/** The archive of a release for a platform: its name without the extension, its address and its pinned sha256. */
export interface Archive {
  name: string;
  url: string;
  sha256: string;
}

/** The archive of `pin` for `platform`; a platform without a pinned sha256 is an error. */
export function archiveOf(pin: Pin, platform: string): Archive {
  const sha256 = pin.sha256[platform];
  if (sha256 === undefined) throw new Error(`config/toolchain.json holds no sha256 of bay for ${platform}`);
  // A release names its archives by the names of Go: the architecture x64 is amd64.
  const name = `bay_${pin.version}_${platform.replace('-', '_').replace(/_x64$/, '_amd64')}`;
  return { name, url: `${pin.releases}/download/${pin.version}/${name}.tar.gz`, sha256 };
}

/**
 * Puts bay and bayd of the archive of `pin` for `platform` into `directory`. `download` gives the bytes of an address.
 * An archive of another sha256, or one without both programs, is an error and changes nothing.
 */
export async function install(pin: Pin, platform: string, directory: string, download: (url: string) => Promise<Buffer>): Promise<void> {
  const archive = archiveOf(pin, platform);
  const bytes = await download(archive.url);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  if (sha256 !== archive.sha256) throw new Error(`the sha256 of ${archive.name}.tar.gz is ${sha256}; config/toolchain.json pins ${archive.sha256}`);
  mkdirSync(dirname(directory), { recursive: true });
  const staging = mkdtempSync(`${directory}.staging-`);
  try {
    const file = join(staging, `${archive.name}.tar.gz`);
    writeFileSync(file, bytes);
    const extracted = spawnSync('tar', ['-xzf', file, '-C', staging], { encoding: 'utf8' });
    if (extracted.status !== 0) throw new Error(`tar does not extract ${archive.name}.tar.gz: ${extracted.stderr}`);
    const programs = ['bay', 'bayd'];
    for (const program of programs) {
      if (!existsSync(join(staging, archive.name, program))) throw new Error(`${archive.name}.tar.gz holds no ${archive.name}/${program}`);
    }
    mkdirSync(directory, { recursive: true });
    for (const program of programs) renameSync(join(staging, archive.name, program), join(directory, program));
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

/** The pin of bay that the tracked file names. */
function pinned(root: string): Pin {
  const config = JSON.parse(readFileSync(join(root, 'config/toolchain.json'), 'utf8')) as { bay?: Partial<Pin> };
  const { version, releases, sha256 } = config.bay ?? {};
  if (typeof version !== 'string' || typeof releases !== 'string' || typeof sha256 !== 'object') {
    throw new Error('config/toolchain.json names no bay with its version, its releases and its sha256');
  }
  return { version, releases, sha256 };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const pin = pinned(root);
  const directory = join(root, 'var/tools/bay');
  try {
    await install(pin, `${process.platform}-${process.arch}`, directory, async (url) => {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`${url} gives the status ${response.status}`);
      return Buffer.from(await response.arrayBuffer());
    });
    console.log(`install-bay: bay ${pin.version} is in ${directory}`);
  } catch (error) {
    console.error(`install-bay: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
