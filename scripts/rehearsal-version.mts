// Prints the version that ci-release-rehearsal rehearses: the first ## X.Y.Z section of the change log, after ## Unreleased.
// It reads CHANGELOG.md itself (or the file named by the first argument) and imports nothing of the kit. A section whose title
// is not exactly X.Y.Z (for example "## 0.0.7 extra") or a change log without one fails with the reason.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const file = process.argv[2] ?? join(root, 'CHANGELOG.md');

function fail(reason: string): never {
  console.error(`rehearsal-version: ${file}: ${reason}`);
  process.exit(2);
}

const titles = readFileSync(file, 'utf8')
  .split('\n')
  .filter((line) => line.startsWith('## '))
  .map((line) => line.slice(3));
const first = titles[0] === 'Unreleased' ? titles[1] : titles[0];
if (first === undefined) fail('no ## section after ## Unreleased, so there is no version to rehearse (no ## X.Y.Z)');
if (!/^\d+\.\d+\.\d+$/.test(first)) fail(`the first version section is "## ${first}", not "## X.Y.Z"; the rehearsal names a version only by an exact section`);
console.log(first);
