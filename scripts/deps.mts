// Clones the checkout of each other polyspec repository whose crate Cargo.lock pins from Git, php-serialization, into
// var/deps/<repository> at that commit (make install-binding), from which the binding test targets build the packages
// of that repository that the generated packages of session-store require: its wheel, its PHP extension, its npm
// package and its Go module (T1.34-1 to T1.34-3). The commit is the one that Cargo.lock pins for the tag of the
// manifests, so the clone is an input fixed by its tag and its commit hash. binding itself is a released npm package,
// so its crate polyspec-binding-core is left out. Only this install target downloads: cargo metadata resolves
// Cargo.lock with --locked and fetches what is missing, and every other target reads the clones offline and fails
// while they are missing.
import { execFileSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const metadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--manifest-path', join(root, 'Cargo.toml')], {
    encoding: 'utf8',
    // The metadata of this workspace, with the crates of Redis and their dependencies, is larger than the default
    // buffer of 1 MiB.
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'inherit'],
  }),
) as { packages: { name: string; source: string | null; manifest_path: string }[] };
let cloned = 0;
for (const dep of metadata.packages) {
  if (dep.source === null || !dep.source.startsWith('git+') || dep.name === 'polyspec-binding-core') continue;
  const commit = /#([0-9a-f]{40})$/.exec(dep.source)?.[1];
  const name = /\/([^/?#]+?)(?:\.git)?[?#]/.exec(dep.source)?.[1];
  if (commit === undefined || name === undefined) throw new Error(`${dep.name} has no pinned commit: ${dep.source}`);
  // The crate is packages/rust of its repository.
  const from = resolve(dirname(dep.manifest_path), '../..');
  const into = join(root, 'var/deps', name);
  cloned += 1;
  const head = existsSync(join(into, '.git')) ? execFileSync('git', ['-C', into, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim() : '';
  if (head === commit) continue;
  rmSync(into, { recursive: true, force: true });
  execFileSync('git', ['clone', '--quiet', '--no-checkout', from, into], { stdio: 'inherit' });
  execFileSync('git', ['-C', into, '-c', 'advice.detachedHead=false', 'checkout', '--quiet', '--detach', commit], { stdio: 'inherit' });
}
if (cloned === 0) throw new Error('Cargo.lock pins no crate of another repository from Git');
