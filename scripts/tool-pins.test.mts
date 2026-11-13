// The pins of the tools of the checkout (AGENTS.md, "Tools"), checked on the tracked files:
//
//   - mise.toml names Node.js and npm by their exact releases, and npm is the release of devEngines of package.json;
//   - mise.lock pins each of them at the release of mise.toml, Node.js with the checksum of its archive of each platform
//     of AGENTS.md, "Platforms";
//   - config/toolchain.json names the release of mise with the sha256 of its archive of each of those platforms;
//   - the bootstrap action sets up the Node.js that mise.toml names, and no .node-version is tracked beside it;
//   - a recipe of the Makefile starts node by its path in var/tools: only the recipes that install that node, bootstrap
//     and install-tools, start the node of the machine.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const EXACT = /^[0-9]+\.[0-9]+\.[0-9]+$/;
// The platform keys of mise.lock and of config/toolchain.json for Linux x64 and arm64 and macOS arm64.
const LOCK_PLATFORMS = ['linux-arm64', 'linux-x64', 'macos-arm64'];
const MISE_PLATFORMS = ['linux-arm64', 'linux-x64', 'darwin-arm64'];
// The recipes that run before the node of var/tools exists.
const MACHINE_NODE = new Set(['bootstrap', 'install-tools']);

const tracked = new Set(execFileSync('git', ['-C', root, 'ls-files', '-z'], { encoding: 'utf8' }).split('\0'));
/** The text of the tracked file `file`; a file that is not tracked fails the test that reads it. */
function read(file: string): string {
  assert.ok(tracked.has(file), `${file} is not tracked`);
  return readFileSync(join(root, file), 'utf8');
}

/** The release of the tool `tool` in the table `[tools]` of the text of a mise.toml; undefined when it names none. */
export function toolRelease(text: string, tool: string): string | undefined {
  const lines = text.split('\n');
  const start = lines.indexOf('[tools]');
  if (start < 0) return undefined;
  for (const line of lines.slice(start + 1)) {
    if (line.startsWith('[')) break;
    const found = /^("?)([^"=\s]+)\1 *= *"([^"]*)"$/.exec(line);
    if (found?.[2] === tool) return found[3];
  }
  return undefined;
}

/** The lines of the entry `[[tools.<key>]]` of the text of a mise.lock with its platform tables, up to the next entry. */
export function lockEntry(text: string, key: string): string[] {
  const lines = text.split('\n');
  const start = lines.indexOf(`[[tools.${key}]]`);
  if (start < 0) return [];
  const end = lines.findIndex((line, index) => index > start && line.startsWith('[['));
  return lines.slice(start + 1, end < 0 ? lines.length : end);
}

// A command of a recipe line whose program is the bare name node: at the start of the line or after `&&`, `;`, `|` or
// the start of a command substitution, behind the variables NAME=value of the command. The word node as an argument,
// such as the tool that run-tests of kit starts, is no such command.
const BARE_NODE = /(?:^[@-]?|&&|;|\||\$\()\s*(?:[A-Za-z_][A-Za-z0-9_]*=\S* +)*node(?: |$)/;

/** The recipe lines of the text of a Makefile that start the node of the machine, each `<target>: <line>`. */
export function machineNode(makefile: string): string[] {
  const found: string[] = [];
  let target = '';
  for (const line of makefile.split('\n')) {
    const name = /^([a-z][A-Za-z0-9_.-]*)(?: +[a-z][A-Za-z0-9_.-]*)* *:(?!=)/.exec(line)?.[1];
    if (name !== undefined) target = name;
    else if (line.startsWith('\t') && BARE_NODE.test(line.trim())) found.push(`${target}: ${line.trim()}`);
  }
  return found;
}

test('the release of a tool is read from the table tools of a mise.toml', () => {
  const text = '# node = "1.0.0"\n[tools]\nnode = "26.12.0"\n"npm:npm" = "12.2.0"\n\n[settings]\ngo = "1.0.0"\n';
  assert.equal(toolRelease(text, 'node'), '26.12.0');
  assert.equal(toolRelease(text, 'npm:npm'), '12.2.0');
  assert.equal(toolRelease(text, 'go'), undefined);
  assert.equal(toolRelease('node = "26.12.0"\n', 'node'), undefined);
});

test('the entry of a tool is read from a mise.lock up to the next entry', () => {
  const text = '[[tools.node]]\nversion = "26.12.0"\n\n[tools.node."platforms.linux-arm64"]\nchecksum = "sha256:ab"\n\n[[tools."npm:npm"]]\nversion = "12.2.0"\n';
  assert.deepEqual(lockEntry(text, 'node'), ['version = "26.12.0"', '', '[tools.node."platforms.linux-arm64"]', 'checksum = "sha256:ab"', '']);
  assert.deepEqual(lockEntry(text, '"npm:npm"'), ['version = "12.2.0"', '']);
  assert.deepEqual(lockEntry(text, 'go'), []);
});

test('a recipe line that starts the node of the machine is found with its target', () => {
  const recipe = ['$(NODE) x.mjs', 'cd var && node --test', 'echo node_modules $(VAR)/node/x', 'node', '$(NODE) run-tests.mjs node -- x.mts', 'cd var && CASES=$(CASES) node --test', 'version="$$(node -p 1)"; echo "$$version"', '@node x.mjs'];
  const makefile = `NODE := $(VAR_TOOLS)/bin/node\nPLATFORM = $(shell node -p 1)\nbootstrap:\n\tnode -p 1\ncheck: a\n\t${recipe.join('\n\t')}\n`;
  assert.deepEqual(machineNode(makefile), [
    'bootstrap: node -p 1',
    'check: cd var && node --test',
    'check: node',
    'check: cd var && CASES=$(CASES) node --test',
    'check: version="$$(node -p 1)"; echo "$$version"',
    'check: @node x.mjs',
  ]);
});

test('mise.toml names Node.js and npm by exact releases, and npm is the release of devEngines', () => {
  const mise = read('mise.toml');
  const node = toolRelease(mise, 'node');
  const npm = toolRelease(mise, 'npm:npm');
  assert.match(node ?? '', EXACT, 'mise.toml names no exact release of node');
  assert.match(npm ?? '', EXACT, 'mise.toml names no exact release of npm:npm');
  const manifest: unknown = JSON.parse(read('package.json'));
  assert.deepEqual((manifest as { devEngines?: unknown }).devEngines, { packageManager: { name: 'npm', version: npm, onFail: 'error' } });
  assert.match(mise, /^locked = true$/m, 'mise.toml does not set locked, so mise would install a tool that mise.lock does not pin');
  assert.match(mise, /^auto_install = false$/m, 'mise.toml does not unset auto_install, so mise would install a tool on its first use');
});

test('mise.lock pins Node.js and npm at the releases of mise.toml, Node.js with a checksum of each platform', () => {
  const mise = read('mise.toml');
  const lock = read('mise.lock');
  const node = lockEntry(lock, 'node');
  assert.ok(node.includes(`version = "${toolRelease(mise, 'node') ?? ''}"`), 'mise.lock does not pin node at the release of mise.toml');
  for (const platform of LOCK_PLATFORMS) {
    const at = node.indexOf(`[tools.node."platforms.${platform}"]`);
    assert.ok(at >= 0, `mise.lock holds no archive of node for ${platform}`);
    assert.match(node[at + 1] ?? '', /^checksum = "sha256:[0-9a-f]{64}"$/, `mise.lock holds no checksum of node for ${platform}`);
  }
  const npm = lockEntry(lock, '"npm:npm"');
  assert.ok(npm.includes(`version = "${toolRelease(mise, 'npm:npm') ?? ''}"`), 'mise.lock does not pin npm:npm at the release of mise.toml');
  assert.ok(npm.some((line) => /digest = "sha256:[0-9a-f]{64}"/.test(line)), 'mise.lock holds no digest of npm:npm');
});

test('config/toolchain.json names the release of mise with the sha256 of its archive of each platform', () => {
  const config: unknown = JSON.parse(read('config/toolchain.json'));
  const mise = (config as { mise?: { version?: unknown; sha256?: Record<string, unknown> } }).mise;
  assert.match(typeof mise?.version === 'string' ? mise.version : '', EXACT, 'config/toolchain.json names no exact release of mise');
  for (const platform of MISE_PLATFORMS) {
    const digest = mise?.sha256?.[platform];
    assert.match(typeof digest === 'string' ? digest : '', /^[0-9a-f]{64}$/, `config/toolchain.json holds no sha256 of mise for ${platform}`);
  }
});

test('config/toolchain.json names the release of bay that CI installs with the sha256 of its archive of each platform, and the Makefile starts bay by one value and reads no version of it', () => {
  const config: unknown = JSON.parse(read('config/toolchain.json'));
  const bay = (config as { bay?: { version?: unknown; releases?: unknown; sha256?: Record<string, unknown> } }).bay;
  assert.match(typeof bay?.version === 'string' ? bay.version : '', /^v[0-9]+\.[0-9]+\.[0-9]+$/, 'config/toolchain.json names no exact release of bay');
  assert.match(typeof bay?.releases === 'string' ? bay.releases : '', /^https:\/\/github\.com\/polyspec\/bay\/releases$/, 'config/toolchain.json names no address of the releases of bay');
  for (const platform of MISE_PLATFORMS) {
    const digest = bay?.sha256?.[platform];
    assert.match(typeof digest === 'string' ? digest : '', /^[0-9a-f]{64}$/, `config/toolchain.json holds no sha256 of bay for ${platform}`);
  }
  // bay of PATH by default, and the bay of var/tools behind sudo -n when BAY_SUDO is set; no recipe names bay itself.
  const makefile = read('Makefile');
  assert.match(makefile, /^BAY \?= \$\(if \$\(BAY_SUDO\),sudo -n \$\(VAR_TOOLS\)\/bay\/bay,bay\)$/m, 'BAY of the Makefile is not bay of PATH or the bay of var/tools behind sudo -n');
  assert.doesNotMatch(makefile, /BAY_VERSION|BAY_RELEASE|toolchain\.json"\)\.bay/, 'the Makefile reads the pin of bay, which only the install of CI reads');
  const started = makefile.split('\n').filter((line) => /^\t(?:.*[ ;|&])?bay(?: |$)/.test(line));
  assert.deepEqual(started, [], 'a recipe starts bay by its name, not by BAY');
});

test('the bootstrap action sets up the Node.js of mise.toml, and no .node-version is tracked', () => {
  assert.ok(!tracked.has('.node-version'), '.node-version is tracked beside mise.toml, a second pin of Node.js');
  const lines = read('.github/actions/bootstrap/action.yml').split('\n').filter((line) => /^ *node-version/.test(line)).map((line) => line.trim());
  assert.deepEqual(lines, ['node-version-file: mise.toml']);
});

test('only the recipes that install the node of var/tools start the node of the machine', () => {
  const makefile = read('Makefile');
  assert.match(makefile, /^NODE := \$\(VAR_TOOLS\)\/bin\/node$/m, 'NODE of the Makefile is not the node of var/tools');
  assert.deepEqual(machineNode(makefile).filter((line) => !MACHINE_NODE.has(line.slice(0, line.indexOf(':')))), []);
});
