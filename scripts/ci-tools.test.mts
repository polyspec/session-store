// Each job target of .github/workflows/ci.yml and of the job assets (assets.yml) installs the mise tools that it calls
// (T1.100): the rule that a job target installs what its own recipe uses, not what an earlier step of the job installed.
//
// For each job target, `make -n` prints the commands of its recipe, of the sub-makes it starts ($(MAKE) lines, printed
// as `make --no-print-directory <target>`) and of the targets that ci-targets runs (the words after `-k var/report/ci-targets`).
// The closure of those targets is walked, and the mise tools whose shim (var/tools/mise/shims/<tool>) any of them runs must
// be installed by the commands of the job: install-tools TOOLS=<names> (quoted or not), install-tools without TOOLS (every
// tool of mise.toml), install-php-tools (composer), install-python-packages (uv), install-wasm-bindgen (wasm-bindgen).
// The PHP of the job is not a mise tool: setup-php of the workflow puts it on the PATH, so it is not checked here.
//
// The recipes that run a test file (run-tests.mjs) are not walked: make -n runs a line that names $(MAKE), so a dry run of
// them would run their tests; and they call no shim of mise by their recipe.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const make = process.env.MAKE ?? 'make';
const makefile = readFileSync(join(root, 'Makefile'), 'utf8');

const jobTargets = ['ci-shared', 'ci-rust', 'ci-test', 'ci-bindings', 'ci-release-rehearsal', 'ci-assets-platform', 'ci-assets-php', 'ci-assets-npm'];

// The recipe lines of a target as the Makefile writes them (its first definition).
function recipeOf(target: string): string {
  const match = makefile.match(new RegExp(`^${target}:[^\\n]*\\n((?:\\t[^\\n]*\\n?)+)`, 'm'));
  return match === null ? '' : match[1];
}

function runsTests(target: string): boolean {
  return recipeOf(target).includes('run-tests.mjs');
}

// The commands that make -n prints for a target, with this file's environment (NODE_TEST_CONTEXT is unset: the test
// runner refuses a test run started below it).
function dryRun(target: string): string {
  const env: Record<string, string | undefined> = { ...process.env, MAKE: make };
  delete env.NODE_TEST_CONTEXT;
  return execFileSync(make, ['-n', '--no-print-directory', '-C', root, target, 'EXPECTED_PHP_MINOR=8.4'], { encoding: 'utf8', env, stdio: ['ignore', 'pipe', 'pipe'] });
}

// The targets that a dry run starts: its sub-makes and the targets of ci-targets.
function startedBy(commands: string): string[] {
  const started: string[] = [];
  for (const line of commands.split('\n')) {
    const sub = line.match(/\bmake --no-print-directory\s+([A-Za-z][\w.-]*)/);
    if (sub) started.push(sub[1]);
    const ci = line.split('ci-targets.mjs -k var/report/ci-targets')[1];
    if (ci !== undefined) started.push(...ci.trim().split(/\s+/).filter((word) => word !== ''));
  }
  return started;
}

// The commands of a job target and of every target it starts, each target walked once.
function closure(job: string): string {
  const seen = new Set<string>();
  const queue = [job];
  let commands = '';
  while (queue.length > 0) {
    const target = queue.shift()!;
    if (seen.has(target) || runsTests(target)) continue;
    seen.add(target);
    const out = dryRun(target);
    commands += out;
    queue.push(...startedBy(out));
  }
  return commands;
}

// The mise tools whose shims the commands run.
function used(commands: string): Set<string> {
  const tools = new Set<string>();
  for (const match of commands.matchAll(/\/mise\/shims\/([a-z0-9-]+)/g)) tools.add(match[1]);
  return tools;
}

// The mise tools that the commands install, or 'all' when one install-tools has no TOOLS.
function installed(commands: string): Set<string> | 'all' {
  const tools = new Set<string>();
  for (const line of commands.split('\n')) {
    if (/\binstall-tools\b/.test(line)) {
      const named = line.match(/\bTOOLS=("([^"]*)"|\S+)/);
      if (named === null) return 'all';
      for (const name of (named[2] ?? named[1]).split(/\s+/)) if (name !== '') tools.add(name);
    }
    if (/\binstall-php-tools\b/.test(line)) tools.add('composer');
    if (/\binstall-python-packages\b/.test(line)) tools.add('uv');
    if (/\binstall-wasm-bindgen\b/.test(line)) tools.add('wasm-bindgen');
  }
  return tools;
}

for (const job of jobTargets) {
  test(`the job target ${job} installs every mise tool that it and the targets it starts call (T1.100)`, () => {
    const commands = closure(job);
    const calls = used(commands);
    const have = installed(commands);
    if (have === 'all') return;
    const missing = [...calls].filter((tool) => !have.has(tool));
    assert.deepEqual(missing, [], `${job} calls ${[...calls].join(', ') || 'no mise tool'} and installs ${[...have].join(', ') || 'none'}`);
  });
}
