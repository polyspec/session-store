// Sets the version of the release on every manifest that config/release-version.json lists (make release-version
// VERSION=X.Y.Z, T1.98): `node scripts/release-version.mts X.Y.Z [--root <directory>]`.
//
// A TOML place is the value of the key `path[last]` in the table `path[0..last-1]`, and a JSON place is the key path of
// the object. The change is made on the parsed table of each file, and a file of the list that does not hold exactly one
// such key fails the run. Before any change, every other manifest of the tree (Cargo.toml, package.json, pyproject.toml)
// that still carries the old version as a version key fails the run too, so no manifest is left at the old version
// without being listed. The locks are not written here: make release-version runs cargo and the pinned npm after this.

import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

interface Site {
  file: string;
  path: string[];
}

const args = process.argv.slice(2);
const rootIndex = args.indexOf('--root');
const root = rootIndex >= 0 ? args[rootIndex + 1] : join(dirname(fileURLToPath(import.meta.url)), '..');
const version = args.find((arg, index) => index !== rootIndex && index !== rootIndex + 1 && !arg.startsWith('--'));

function fail(reason: string): never {
  console.error(`release-version: ${reason}`);
  process.exit(2);
}

if (version === undefined || !/^\d+\.\d+\.\d+$/.test(version)) fail(`VERSION must be X.Y.Z, not "${version ?? ''}"`);

const config = JSON.parse(readFileSync(join(root, 'config', 'release-version.json'), 'utf8')) as { sites: Site[] };
const sites = config.sites;
if (sites.length === 0) fail('config/release-version.json lists no place');

// The version before the change: the value of the first place, which every other place must carry too.
function read(site: Site): string {
  const text = readFileSync(join(root, site.file), 'utf8');
  if (site.file.endsWith('.json')) {
    const value = (JSON.parse(text) as Record<string, unknown>)[site.path[0]];
    if (typeof value !== 'string') fail(`${site.file}: no string key "${site.path[0]}"`);
    return value;
  }
  const found = tomlPlaces(text, site.path);
  if (found.length !== 1) fail(`${site.file}: ${found.length} places of "${site.path.join('.')}", not one`);
  return found[0].value;
}

interface Place { line: number; value: string }

// The lines of the version key of the table that the path names: the key of the table header that equals path[0..-1].
function tomlPlaces(text: string, path: string[]): Place[] {
  const table = path.slice(0, -1).join('.');
  const key = path[path.length - 1];
  const lines = text.split('\n');
  const places: Place[] = [];
  let current: string | undefined;
  lines.forEach((line, index) => {
    const header = line.match(/^\s*\[([^\[\]]+)\]\s*$/);
    if (header) {
      current = header[1].trim();
      return;
    }
    if (current !== table) return;
    const match = line.match(new RegExp(`^(\\s*)${key}\\s*=\\s*"([^"]*)"\\s*$`));
    if (match) places.push({ line: index, value: match[2] });
  });
  return places;
}

const oldVersion = read(sites[0]);
for (const site of sites) {
  if (read(site) !== oldVersion) fail(`${site.file} carries ${read(site)}, not the version ${oldVersion} of the other places`);
}

// Every tracked-like manifest of the tree that carries the old version, outside the list, fails the run before any write.
const listed = new Set(sites.map((site) => site.file));
const skip = new Set(['node_modules', '.git', 'var', 'target', '.venv', 'dist']);
const stray: string[] = [];
function walk(dir: string): void {
  for (const entry of readdirSync(dir)) {
    if (skip.has(entry)) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      walk(full);
      continue;
    }
    if (!/(^|\/)(Cargo\.toml|package\.json|pyproject\.toml)$/.test(entry)) continue;
    const rel = relative(root, full);
    if (listed.has(rel)) continue;
    const text = readFileSync(full, 'utf8');
    const holds = rel.endsWith('.json')
      ? (JSON.parse(text) as Record<string, unknown>).version === oldVersion
      : new RegExp(`^\\s*version\\s*=\\s*"${oldVersion.replace(/\./g, '\\.')}"\\s*$`, 'm').test(text);
    if (holds) stray.push(rel);
  }
}
walk(root);
if (stray.length > 0) fail(`${stray.join(', ')} carry the old version ${oldVersion} outside config/release-version.json; list them there or change them by hand first`);

// The change: the value of each place, written in its file; a TOML file takes its line, a JSON file its object again.
for (const site of sites) {
  const full = join(root, site.file);
  if (site.file.endsWith('.json')) {
    const original = readFileSync(full, 'utf8');
    const object = JSON.parse(original) as Record<string, unknown>;
    object[site.path[0]] = version;
    writeFileSync(full, `${JSON.stringify(object, null, 2)}\n`);
    continue;
  }
  const text = readFileSync(full, 'utf8');
  const [place] = tomlPlaces(text, site.path);
  const lines = text.split('\n');
  lines[place.line] = lines[place.line].replace(/"[^"]*"/, `"${version}"`);
  writeFileSync(full, lines.join('\n'));
}

console.log(`release-version: ${sites.length} places ${oldVersion} -> ${version}`);
