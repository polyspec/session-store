// The rules of the GitHub workflows of this repository (AGENTS.md, "CI and release"), checked on the tracked files of
// .github and on the Makefile:
//
//   - every job runs on ubuntu-26.04-arm;
//   - every action is a local action of .github/actions or is pinned by its full commit, and a comment of the file names
//     its release, `# <owner>/<repository> <release>`;
//   - no job and no step sets timeout-minutes;
//   - every run step is one make target that the Makefile defines, with arguments NAME=value or one expression alone,
//     so that the settings and the checks of the Makefile apply to every command of CI;
//   - the workflow ci starts on a push of main and on workflow_dispatch alone.
//
// The test reads the files as text, line by line: the workflows hold one key on a line, and a run step whose command
// continues on a later line is a finding.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const RUNNER = 'ubuntu-26.04-arm';
const CI = '.github/workflows/ci.yml';
const RELEASE = '.github/workflows/release.yml';

// A run step: make, one target, then arguments NAME=value or expressions ${{ ... }}.
const RUN = /^make ([a-z][a-z0-9-]*)((?: (?:[A-Z][A-Z0-9_]*=[A-Za-z0-9_.-]+|\$\{\{[^}]*\}\}))*)$/;
const PINNED = /^([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)@[0-9a-f]{40}$/;

/** The targets that the text of a Makefile defines with a rule. */
function makeTargets(makefile: string): Set<string> {
  const targets = new Set<string>();
  for (const line of makefile.split('\n')) {
    const names = /^([a-z][A-Za-z0-9_.-]*(?: +[a-z][A-Za-z0-9_.-]*)*) *:(?!=)/.exec(line)?.[1];
    if (names !== undefined) for (const name of names.split(/ +/)) targets.add(name);
  }
  return targets;
}

/** The value of the key `key` of a line, `key: value` or `- key: value`, without its quotes; undefined for another line. */
function valueOf(line: string, key: string): string | undefined {
  const found = new RegExp(`^ *(?:- )?${key}: *(.*)$`).exec(line)?.[1];
  return found?.trim();
}

/**
 * The findings of the workflow or action `file` with the text `text`, each `<file>:<line>: <finding>`. `targets` are the
 * targets of the Makefile and `actions` the local actions, `./.github/actions/<name>`.
 */
function findings(file: string, text: string, targets: Set<string>, actions: Set<string>): string[] {
  const found: string[] = [];
  const lines = text.split('\n');
  const comments = lines.filter((line) => line.trim().startsWith('#'));
  let runners = 0;
  lines.forEach((line, index) => {
    if (line.trim().startsWith('#')) return;
    const at = `${file}:${String(index + 1)}`;
    if (/^ *(?:- )?timeout-minutes:/.test(line)) found.push(`${at}: sets timeout-minutes`);
    const runner = valueOf(line, 'runs-on');
    if (runner !== undefined) {
      runners += 1;
      if (runner !== RUNNER) found.push(`${at}: runs on ${runner}, not on ${RUNNER}`);
    }
    const uses = valueOf(line, 'uses');
    if (uses !== undefined) {
      if (uses.startsWith('./')) {
        if (!actions.has(uses)) found.push(`${at}: uses ${uses}, which is no tracked action`);
      } else {
        const name = PINNED.exec(uses)?.[1];
        if (name === undefined) found.push(`${at}: uses ${uses}, which is not pinned by a full commit`);
        else if (!comments.some((comment) => new RegExp(`# ${name.replaceAll('.', '\\.')} v?[0-9]+\\.[0-9]+\\.[0-9]+\\b`).test(comment)))
          found.push(`${at}: no comment names the release of ${name}`);
      }
    }
    const run = valueOf(line, 'run');
    const next = lines.slice(index + 1).find((later) => later.trim() !== '' && !later.trim().startsWith('#'));
    const deeper = next !== undefined && next.search(/\S/) > line.indexOf('run:');
    // A key `run:` with no value whose deeper lines are keys is a mapping of that name, such as the input `run` of a
    // dispatch, and no run step.
    if (run !== undefined && !(run === '' && deeper && /^ *[A-Za-z_-]+:(?: |$)/.test(next))) {
      const target = RUN.exec(run)?.[1];
      if (target === undefined) found.push(`${at}: the run step is not one make target: ${run}`);
      else if (!targets.has(target)) found.push(`${at}: the Makefile defines no target ${target}`);
      if (deeper) found.push(`${at}: the run step continues on a later line`);
    }
  });
  if (file.includes('/workflows/') && runners === 0) found.push(`${file}: names no runner`);
  return found;
}

/** The findings of the triggers of the workflow ci: the keys of its block `on:` are push, of main alone, and workflow_dispatch. */
function triggerFindings(file: string, text: string): string[] {
  const lines = text.split('\n');
  const start = lines.indexOf('on:');
  if (start < 0) return [`${file}: has no block on:`];
  const block: string[] = [];
  for (const line of lines.slice(start + 1)) {
    if (/^\S/.test(line) && !line.startsWith('#')) break;
    if (line.trim() !== '' && !line.trim().startsWith('#')) block.push(line);
  }
  const indent = Math.min(...block.map((line) => line.search(/\S/)));
  const keys = block.filter((line) => line.search(/\S/) === indent).map((line) => line.trim().replace(/:.*$/, '')).sort();
  const found: string[] = [];
  if (keys.join(' ') !== 'push workflow_dispatch') found.push(`${file}: starts on ${keys.join(', ') || 'nothing'}, not on push and workflow_dispatch alone`);
  if (block.filter((line) => /^ *branches:/.test(line)).map((line) => line.trim()).join(' ') !== 'branches: [main]') found.push(`${file}: the push is not of main alone`);
  return found;
}

const TARGETS = new Set(['docs-check', 'release-tag']);
const ACTIONS = new Set(['./.github/actions/bootstrap']);
const GOOD = `# actions/checkout v7.0.1: the checkout.
name: ci
on:
  push:
    branches: [main]
  workflow_dispatch:
jobs:
  shared:
    runs-on: ubuntu-26.04-arm
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          fetch-depth: 0
      - uses: ./.github/actions/bootstrap
      - name: documents
        run: make docs-check
      - run: make release-tag RUN=1 \${{ github.event_name == 'workflow_run' && 'AUTO=1' || '' }}
`;

test('a workflow that keeps every rule has no finding', () => {
  assert.deepEqual(findings(CI, GOOD, TARGETS, ACTIONS), []);
  assert.deepEqual(triggerFindings(CI, GOOD), []);
  // The input `run` of a dispatch is a mapping, no run step.
  assert.deepEqual(findings(CI, GOOD.replace('  workflow_dispatch:\n', '  workflow_dispatch:\n    inputs:\n      run:\n        required: true\n'), TARGETS, ACTIONS), []);
});

test('each broken rule of a workflow is a finding', () => {
  const broken = (from: string, to: string): string[] => {
    assert.ok(GOOD.includes(from), `the sample holds no ${from}`);
    return findings(CI, GOOD.replace(from, to), TARGETS, ACTIONS);
  };
  assert.deepEqual(broken('runs-on: ubuntu-26.04-arm', 'runs-on: ubuntu-24.04'), [`${CI}:9: runs on ubuntu-24.04, not on ${RUNNER}`]);
  assert.deepEqual(broken('    runs-on: ubuntu-26.04-arm\n', ''), [`${CI}: names no runner`]);
  assert.deepEqual(broken('    steps:', '    timeout-minutes: 10\n    steps:'), [`${CI}:10: sets timeout-minutes`]);
  assert.deepEqual(broken('@3d3c42e5aac5ba805825da76410c181273ba90b1', '@v7'), [`${CI}:11: uses actions/checkout@v7, which is not pinned by a full commit`]);
  assert.deepEqual(broken('@3d3c42e5aac5ba805825da76410c181273ba90b1', '@3d3c42e'), [`${CI}:11: uses actions/checkout@3d3c42e, which is not pinned by a full commit`]);
  assert.deepEqual(broken('# actions/checkout v7.0.1: the checkout.', '# The checkout.'), [`${CI}:11: no comment names the release of actions/checkout`]);
  assert.deepEqual(broken('./.github/actions/bootstrap', './.github/actions/other'), [`${CI}:14: uses ./.github/actions/other, which is no tracked action`]);
  assert.deepEqual(broken('run: make docs-check', 'run: node scripts/check.mts'), [`${CI}:16: the run step is not one make target: node scripts/check.mts`]);
  assert.deepEqual(broken('run: make docs-check', 'run: make docs-check && make release-tag'), [`${CI}:16: the run step is not one make target: make docs-check && make release-tag`]);
  assert.deepEqual(broken('run: make docs-check', 'run: make docs-check release-tag'), [`${CI}:16: the run step is not one make target: make docs-check release-tag`]);
  assert.deepEqual(broken('run: make docs-check', 'run: make test'), [`${CI}:16: the Makefile defines no target test`]);
  assert.deepEqual(broken('run: make docs-check', 'run: |\n          make docs-check'), [`${CI}:16: the run step is not one make target: |`, `${CI}:16: the run step continues on a later line`]);
  assert.deepEqual(broken('run: make docs-check', 'run:\n          make docs-check && make release-tag'), [`${CI}:16: the run step is not one make target: `, `${CI}:16: the run step continues on a later line`]);
  assert.deepEqual(broken('run: make docs-check', 'run: make docs-check\n          make release-tag'), [`${CI}:16: the run step continues on a later line`]);
});

test('each trigger of the workflow ci other than a push of main and workflow_dispatch is a finding', () => {
  assert.deepEqual(triggerFindings(CI, GOOD.replace('on:\n', 'on:\n  pull_request:\n')), [`${CI}: starts on pull_request, push, workflow_dispatch, not on push and workflow_dispatch alone`]);
  assert.deepEqual(triggerFindings(CI, GOOD.replace('  workflow_dispatch:\n', '')), [`${CI}: starts on push, not on push and workflow_dispatch alone`]);
  assert.deepEqual(triggerFindings(CI, GOOD.replace('branches: [main]', 'branches: [main, next]')), [`${CI}: the push is not of main alone`]);
  assert.deepEqual(triggerFindings(CI, GOOD.replace('    branches: [main]\n', '')), [`${CI}: the push is not of main alone`]);
  assert.deepEqual(triggerFindings(CI, GOOD.replace('on:\n', 'on: [push]\n')), [`${CI}: has no block on:`]);
});

test('the Makefile rules name their targets', () => {
  assert.deepEqual([...makeTargets('A := b\n.PHONY: x\nx y: z\n\trun: it\nbindings-%: w\ntest-php: export PHP := php\n')].sort(), ['test-php', 'x', 'y']);
});

test('the workflows and the actions of the repository keep the rules', () => {
  const tracked = execFileSync('git', ['-C', root, 'ls-files', '-z', '--', '.github'], { encoding: 'utf8' }).split('\0').filter((file) => file !== '');
  const workflows = tracked.filter((file) => /^\.github\/workflows\/[^/]+\.yml$/.test(file));
  const actionFiles = tracked.filter((file) => /^\.github\/actions\/[^/]+\/action\.yml$/.test(file));
  assert.deepEqual(tracked.filter((file) => !workflows.includes(file) && !actionFiles.includes(file)), [], 'a tracked file of .github is no workflow and no action');
  for (const file of [CI, RELEASE]) assert.ok(workflows.includes(file), `${file} is not tracked`);
  const targets = makeTargets(readFileSync(join(root, 'Makefile'), 'utf8'));
  const actions = new Set(actionFiles.map((file) => `./${dirname(file)}`));
  const found = [...workflows, ...actionFiles].flatMap((file) => findings(file, readFileSync(join(root, file), 'utf8'), targets, actions));
  found.push(...triggerFindings(CI, readFileSync(join(root, CI), 'utf8')));
  assert.deepEqual(found, []);
});
