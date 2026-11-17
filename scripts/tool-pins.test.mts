// The pins of the tools of the checkout (AGENTS.md, "Tools"), checked on the tracked files:
//
//   - mise.toml names Node.js and npm by their exact releases, and npm is the release of devEngines of package.json;
//   - mise.lock pins each of them at the release of mise.toml, Node.js with the checksum of its archive of each platform
//     of AGENTS.md, "Platforms";
//   - config/toolchain.json names the release of mise with the sha256 of its archive of each of those platforms;
//   - the bootstrap action sets up the Node.js that mise.toml names, and no .node-version is tracked beside it;
//   - a recipe of the Makefile starts node by its path in var/tools: only the recipes that install that node, bootstrap
//     and install-tools, start the node of the machine;
//   - binding is the npm package of its release, pinned by the URL of its asset and its integrity, whose tag the
//     manifests name for polyspec-binding-core, and the binding targets run Cargo offline.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
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
  // bay of PATH, with no sudo: the action bay of CI puts the bay of var/tools on PATH; no recipe names bay itself.
  const makefile = read('Makefile');
  assert.match(makefile, /^BAY \?= bay$/m, 'BAY of the Makefile is not bay of PATH');
  assert.doesNotMatch(makefile, /\bsudo\b|BAY_SUDO/, 'the Makefile starts a command with sudo, which runs bay as another user than the run');
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

test('binding is the release that package.json pins by its asset and the manifests by its tag, run offline', () => {
  const pkg = JSON.parse(read('package.json')) as { devDependencies: Record<string, string> };
  const url = pkg.devDependencies['@polyspec/binding'];
  const asset = /^https:\/\/github\.com\/polyspec\/binding\/releases\/download\/v([0-9]+\.[0-9]+\.[0-9]+)\/polyspec-binding-\1\.tgz$/.exec(url ?? '');
  assert.ok(asset !== null, `package.json pins @polyspec/binding by ${url}, not by the URL of a release asset`);
  const version = asset[1];
  const lock = JSON.parse(read('package-lock.json')) as { packages: Record<string, { version?: string; resolved?: string; integrity?: string }> };
  const entry = lock.packages['node_modules/@polyspec/binding'];
  assert.equal(entry?.version, version);
  assert.equal(entry?.resolved, url);
  assert.match(entry?.integrity ?? '', /^sha512-/, 'package-lock.json pins @polyspec/binding without its integrity');
  const lines = execFileSync('git', ['-C', root, 'grep', '-h', '^polyspec-binding-core', '--', '*Cargo.toml'], { encoding: 'utf8' }).trim().split('\n');
  assert.ok(lines.length > 0, 'no manifest names polyspec-binding-core');
  for (const line of lines) assert.match(line, new RegExp(`git = "https://github.com/polyspec/binding.git", tag = "v${version.replaceAll('.', '\\.')}"`), `${line} names no tag v${version}`);
  const makefile = read('Makefile');
  assert.match(makefile, /^BINDING := \$\(NODE\) \$\(CURDIR\)\/node_modules\/@polyspec\/binding\/dist\/cli\.js$/m, 'BINDING of the Makefile is not the installed package');
  assert.match(makefile, /^binding-check \$\(addprefix bindings-,\$\(TARGETS\)\): export CARGO_NET_OFFLINE := true$/m, 'the binding targets do not run Cargo offline');
  assert.doesNotMatch(makefile + read('.github/workflows/ci.yml'), /install-binding/, 'a recipe or a job still installs binding apart from the packages');
  // The clones of the other repositories that Cargo.lock pins from Git are fetched by install-deps (scripts/deps.mts),
  // which installs no binding (T1.34-1).
  assert.match(makefile, /^install-deps:$/m, 'no target install-deps fetches the clones of scripts/deps.mts');
});

test('the local links are the targets link, unlink and link-check of kit, and .links is ignored', () => {
  // The local links of 0.0.x development (kit local-links.md, K869): .links names the checkouts that provide packages.
  const makefile = read('Makefile');
  assert.match(makefile, /^link:.*\n\t\$\(NODE\) \$\(KIT\)\/link\.mjs link\n/m, 'no target link runs link.mjs link');
  assert.match(makefile, /^unlink:.*\n\t\$\(NODE\) \$\(KIT\)\/link\.mjs unlink\n/m, 'no target unlink runs link.mjs unlink');
  assert.match(makefile, /^link-check:.*\n\t\$\(NODE\) \$\(KIT\)\/link-check\.mjs\n/m, 'no target link-check runs link-check.mjs');
  assert.match(read('.gitignore'), /^\/\.links$/m, '.gitignore does not ignore /.links');
  assert.ok(!tracked.has('.links'), '.links is tracked');
  // link writes a go.work beside each go.mod of the consumer, which Git must ignore there.
  assert.match(read('.gitignore'), /^go\.work$/m, '.gitignore does not ignore the go.work of link');
  assert.match(read('.gitignore'), /^go\.work\.sum$/m, '.gitignore does not ignore the go.work.sum of link');
});

test('a php-serialization of .links is the checkout that the targets build, and none generates into it', () => {
  // Linked development (kit K869): the checkout that .links names builds the packages of php-serialization in place of
  // the clone of the release in var/deps, and without it the clone stays.
  const make = process.env.MAKE ?? 'make';
  const print = (links: string): string => execFileSync(make, ['-s', '--no-print-directory', '-C', root, `LINKS_FILE=${links}`, '--eval', 'print-ps: ; @echo $(PHP_SERIALIZATION)', 'print-ps'], { encoding: 'utf8', env: { ...process.env, MAKEFLAGS: '' } }).trim();
  // A checkout is php-serialization by the crate of its packages/rust, whatever its folder is named, as the folder of a
  // worktree of a lane is.
  const directory = mkdtempSync(join(tmpdir(), 'links-'));
  const lane = join(directory, 'php-serialization-lane');
  mkdirSync(join(lane, 'packages/rust'), { recursive: true });
  writeFileSync(join(lane, 'packages/rust/Cargo.toml'), '[package]\nname = "polyspec-php-serialization"\n');
  mkdirSync(join(directory, 'binding'));
  const links = join(directory, 'links');
  writeFileSync(links, `${join(directory, 'binding')}\n${lane}\n`);
  assert.equal(print(links), lane);
  // The checkout of .links is a provider that the run never writes into (T1.54): its provider generates its bindings in
  // it, so no target of php-serialization runs binding generate there, and the clone of the release is generated.
  const plan = (file: string, target: string): string => execFileSync(make, ['-n', '--no-print-directory', '-C', root, `LINKS_FILE=${file}`, target], { encoding: 'utf8', env: { ...process.env, MAKEFLAGS: '' } });
  for (const target of ['php-serialization-python', 'php-serialization-php', 'php-serialization-go']) {
    assert.doesNotMatch(plan(links, target), /cli\.js generate/, `${target} generates into the checkout of .links`);
  }
  writeFileSync(links, `${join(directory, 'binding')}\n`);
  assert.equal(print(links), join(root, 'var/deps/php-serialization'));
  assert.match(plan(links, 'php-serialization-go'), /cli\.js generate --root \S*var\/deps\/php-serialization --target go/);
  rmSync(directory, { recursive: true });
});

test('every cargo of the recipes runs through the cargo of the link of kit', () => {
  // Linked development (kit K869): `link.mjs cargo <dir>` patches the checkouts of .links into one cargo command, and
  // without .links runs cargo as given; a recipe that ran cargo itself would build the tags of the lock.
  const makefile = read('Makefile');
  assert.match(makefile, /^CARGO := \$\(NODE\) \$\(KIT\)\/link\.mjs cargo \$\(CURDIR\)$/m, 'CARGO is not the cargo of the link of kit');
  const own = makefile.split('\n').filter((line) => line.startsWith('\t') && /(^|[\s;&|(])cargo /.test(line.replace(/\$\(CARGO\)/g, '')));
  assert.deepEqual(own, [], 'a recipe runs cargo itself');
  // maturin runs cargo itself, so it runs through `link.mjs maturin <dir>` of kit with the maturin of the checkout (K1266).
  assert.match(makefile, /^MATURIN_LINK := MATURIN='\$\(MATURIN\)' \$\(NODE\) \$\(KIT\)\/link\.mjs maturin \$\(CURDIR\)$/m, 'no MATURIN_LINK runs maturin through the link of kit');
  const maturin = makefile.split('\n').filter((line) => line.startsWith('\t') && /\$\(MATURIN\) /.test(line));
  assert.deepEqual(maturin, [], 'a recipe runs maturin itself');
});

test('packages/ts is a workspace of the root, whose php-serialization is the npm package of its release', () => {
  // make install-packages installs the dependencies of packages/ts with the root lock, so that kit links them (K888); the
  // release is the one that the manifests of Cargo name for php-serialization.
  const root_ = JSON.parse(read('package.json')) as { workspaces?: string[] };
  assert.deepEqual(root_.workspaces, ['packages/ts']);
  const tag = /polyspec\/php-serialization\.git", tag = "v([0-9.]+)"/.exec(read('packages/redis/rust/Cargo.toml'))?.[1];
  assert.ok(tag !== undefined, 'no tag of php-serialization in packages/redis/rust/Cargo.toml');
  const url = `https://github.com/polyspec/php-serialization/releases/download/v${tag}/polyspec-php-serialization-${tag}.tgz`;
  // The published manifest requires php-serialization by its version, and the root, which installs it, names the asset of
  // that release (binding B2.405, T1.59).
  const ts = JSON.parse(read('packages/ts/package.json')) as { dependencies?: Record<string, string> };
  assert.equal(ts.dependencies?.['@polyspec/php-serialization'], tag);
  const top = JSON.parse(read('package.json')) as { devDependencies?: Record<string, string> };
  assert.equal(top.devDependencies?.['@polyspec/php-serialization'], url);
  const lock = JSON.parse(read('package-lock.json')) as { packages: Record<string, { resolved?: string; integrity?: string }> };
  const locked = lock.packages['node_modules/@polyspec/php-serialization'];
  assert.equal(locked?.resolved, url);
  assert.match(locked?.integrity ?? '', /^sha512-/);
  // T1.76: the addon of php-serialization is the npm package of the platform of its release, which the root names by the
  // URL of its asset in optionalDependencies (binding GN-3c), so the install of the root holds it and nothing builds it.
  const optional = (JSON.parse(read('package.json')) as { optionalDependencies?: Record<string, string> }).optionalDependencies ?? {};
  for (const platform of ['darwin-arm64', 'linux-arm64-gnu', 'linux-x64-gnu']) {
    const name = `@polyspec/php-serialization-${platform}`;
    const asset = `https://github.com/polyspec/php-serialization/releases/download/v${tag}/polyspec-php-serialization-${platform}-${tag}.tgz`;
    assert.equal(optional[name], asset, `the root does not name the asset of ${name}`);
    assert.equal(lock.packages[`node_modules/${name}`]?.resolved, asset);
    assert.match(lock.packages[`node_modules/${name}`]?.integrity ?? '', /^sha512-/);
  }
  // The generated tests of Node and of WASM run on that install, which kit links, with the addon of the checkout beside
  // the generated entry: no recipe packs a package of php-serialization or installs a second copy into var.
  const makefile = read('Makefile');
  assert.doesNotMatch(makefile, /\$\(VAR\)\/node\b|\$\(VAR\)\/deps\/npm|php-serialization-node/, 'a recipe installs a copy of the Node packages into var');
  assert.match(makefile, /^\t\$\(SERVICE_RUN\) \$\(NODE\) --test packages\/ts\/generated\/tests\/generated\.test\.mjs$/m, 'test-node does not run the generated tests in the checkout');
  assert.match(makefile, /^\t\$\(NODE\) --expose-gc --test packages\/ts\/wasm\/generated\/tests\/generated\.test\.mjs$/m, 'test-wasm does not run the generated tests in the checkout');
});

test('the hook commit-msg of .githooks refuses a message with the footer of an AI tool', () => {
  // T1.61: git runs .githooks/commit-msg, an executable file, with the file of the message; it runs commit-msg-hook,
  // the rules commit-ai-footer and commit-ai-identity of check-commits of kit, which refuses the footer and passes a
  // message without it. make hooks names the hook, so hooks-check fails a checkout whose core.hooksPath misses it.
  const mode = execFileSync('git', ['-C', root, 'ls-files', '-s', '.githooks/commit-msg'], { encoding: 'utf8' }).split(' ')[0];
  assert.equal(mode, '100755', '.githooks/commit-msg is not tracked as an executable file');
  assert.match(read('.githooks/commit-msg'), /^exec make --no-print-directory commit-msg-hook MESSAGE="\$1"$/m);
  assert.match(read('Makefile'), /^HOOK_DECLARATION := --hooks-path \.githooks --hook commit-msg /m);
  const make = process.env.MAKE ?? 'make';
  const directory = mkdtempSync(join(tmpdir(), 'message-'));
  const check = (text: string): number => {
    const message = join(directory, 'message');
    writeFileSync(message, text);
    // The git of the runner of CI has no global configuration and no identity it can guess, so the run reads none, and
    // gives the identity of a person, as git commit does, which refuses to commit without one; commit-ai-identity reads it.
    const runner = {
      GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_COUNT: '1', GIT_CONFIG_KEY_0: 'user.useConfigOnly', GIT_CONFIG_VALUE_0: 'true',
      GIT_AUTHOR_NAME: 'A Person', GIT_AUTHOR_EMAIL: 'person@example.com', GIT_COMMITTER_NAME: 'A Person', GIT_COMMITTER_EMAIL: 'person@example.com',
    };
    const run = spawnSync(make, ['-s', '--no-print-directory', '-C', root, 'commit-msg-hook', `MESSAGE=${message}`], { encoding: 'utf8', env: { ...process.env, MAKEFLAGS: '', ...runner } });
    return run.status ?? -1;
  };
  assert.notEqual(check('test: x\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n'), 0, 'the hook passes the footer of an AI tool');
  assert.equal(check('test: x\n'), 0, 'the hook refuses a message without a footer');
  rmSync(directory, { recursive: true });
});

test('the musl targets of the Go libraries build C with the clang and llvm-ar of the pinned LLVM', () => {
  // T1.68: ring, the cryptography of rustls (SR-6.1), builds C for the musl target of release-go-static on a Linux host,
  // and the cc crate finds no <arch>-linux-musl-gcc on a runner; the Makefile declares the C compiler and the archiver of
  // each musl target of rust-toolchain.toml as the clang and the llvm-ar of the LLVM of config/toolchain.json, by their
  // absolute paths, and the job platform checks that clang before it builds.
  const makefile = read('Makefile');
  for (const target of ['x86_64_unknown_linux_musl', 'aarch64_unknown_linux_musl']) {
    assert.match(makefile, new RegExp(`^export CC_${target} = \\$\\(LLVM_ROOT\\)/bin/clang$`, 'm'), `CC_${target} is not the clang of LLVM_ROOT`);
    assert.match(makefile, new RegExp(`^export AR_${target} = \\$\\(LLVM_ROOT\\)/bin/llvm-ar$`, 'm'), `AR_${target} is not the llvm-ar of LLVM_ROOT`);
  }
  const platform = makefile.split('\nci-assets-platform:\n')[1]?.split('\n\n')[0] ?? '';
  assert.match(platform, /\$\(MAKE\) --no-print-directory musl-cc-check\n/, 'the job platform does not check the clang of the musl targets');
});
