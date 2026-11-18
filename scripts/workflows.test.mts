// The rules of the GitHub workflows of this repository (AGENTS.md, "CI and release"), checked on the tracked files of
// .github and on the Makefile:
//
//   - every job runs on ubuntu-26.04-arm, except the job platform of assets.yml, which runs on the runner of each platform
//     of the release, ubuntu-26.04, ubuntu-26.04-arm or macos-15, because the asset of a platform is linked and loaded only
//     on that platform: no tool of the release compiles for another platform (binding GN-19a);
//   - the workflows ci and release call assets.yml, which builds the assets of each platform (binding GN-19i);
//   - the workflow ci has the job bindings, which runs `make ci-bindings`, and CI_BINDINGS of the Makefile names the
//     tests of every target of the contract (binding BD-27): lint-bindings, check-declarations, test-python, test-node,
//     test-wasm, test-php and test-go, so that no binding package is released untested; the job starts the Redis
//     service of the generated tests as the job test starts its servers (SE-10.7): it uses the action bay, which gives bay
//     and its machine setup, before its run step `make ci-bindings`, whose recipe runs redis-image before the tests and
//     applies no machine setup of bay: no recipe runs setup-bay or sudo;
//   - every action is a local action of .github/actions or is pinned by its full commit, and a comment of the file names
//     its release, `# <owner>/<repository> <release>`;
//   - no job and no step sets timeout-minutes;
//   - every run step is one make target that the Makefile or the release recipes that it includes (packages/release.mk,
//     which binding release-recipes writes) define, with arguments NAME=value or one expression alone,
//     so that the settings and the checks of the Makefile apply to every command of CI;
//   - the workflow ci starts on a push of main and on workflow_dispatch alone;
//   - the last job of the workflow ci is ci-passed, which runs after every other job, also after a failed one
//     (`if: ${{ always() }}`), needs each of them and runs `make ci-passed FAIL_SKIPPED=1 RESULTS='${{ toJSON(needs) }}'`,
//     the JSON of its needs, so that it passes only when every other job succeeded;
//   - every job of the workflows ci, release and assets, which both call, has one run step, `make ci-<job>`,
//     `make ci-release-<job>` or `make ci-assets-<job>`, with make variables alone, a matrix value as
//     NAME=${{ matrix.<key> }}, an input of the called workflow as NAME=${{ inputs.<key> }}, an output of a needed job as
//     NAME=${{ needs.<job>.outputs.<key> }} and a quoted expression of the event as NAME="${{ ... }}"; its other steps are
//     uses steps, only a uses step has a condition, no step sets env and a job sets GH_TOKEN alone, so that the recipe
//     holds the order and the conditions of the job and a local run is the call that the workflow makes.
//
// The test reads the files as text, line by line: the workflows hold one key on a line, and a run step whose command
// continues on a later line is a finding.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const RUNNER = 'ubuntu-26.04-arm';
const CI = '.github/workflows/ci.yml';
const RELEASE = '.github/workflows/release.yml';
const ASSETS = '.github/workflows/assets.yml';
// The runner of each platform of the release, on which the job platform of assets.yml builds the assets of that platform
// (binding GN-19a).
const PLATFORM_RUNNERS: Record<string, string> = { 'linux-x64-gnu': 'ubuntu-26.04', 'linux-arm64-gnu': 'ubuntu-26.04-arm', 'darwin-arm64': 'macos-15' };
const MATRIX_RUNNER = '${{ matrix.runner }}';
const LAST = 'ci-passed';

// A run step: make, one target, then arguments NAME=value, NAME="words" for a list such as the tools of a job, or
// expressions ${{ ... }}, bare or quoted.
const RUN = /^make ([a-z][a-z0-9-]*)((?: (?:[A-Z][A-Z0-9_]*=(?:[A-Za-z0-9_.-]+|"[A-Za-z0-9_. -]+"|\$\{\{[^}]*\}\}|'\$\{\{[^}]*\}\}'|"\$\{\{[^}"]*\}\}")|\$\{\{[^}]*\}\}))*)$/;
// An argument of the run step of a job: NAME=value, NAME="words", a matrix value or an input of the called workflow,
// NAME=${{ matrix.<key> }} or NAME=${{ inputs.<key> }}, an output of a job that the job needs, NAME=${{ needs.<job>.outputs.<key> }},
// or a quoted expression of the event, NAME="${{ ... }}", such as the run id and the AUTO of the job tag of release.yml.
const JOB_ARGUMENT = /^[A-Z][A-Z0-9_]*=(?:[A-Za-z0-9_.-]+|"[A-Za-z0-9_. -]+"|\$\{\{ (?:matrix|inputs)\.[a-z][a-z0-9_-]* \}\}|\$\{\{ needs\.[a-z][a-z0-9_-]*\.outputs\.[a-z][a-z0-9_-]* \}\}|"\$\{\{ [^"}]* \}\}")$/;
// The one environment that a job may set: the token of the run, which the release targets read with gh.
const JOB_ENV = ['    env:', '      GH_TOKEN: ${{ github.token }}'];
// The run step of the judgement job ci-passed, which reads the results of the other jobs.
const LAST_RUN = `make ${LAST} FAIL_SKIPPED=1 RESULTS='\${{ toJSON(needs) }}'`;
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
      // The runner of the matrix is judged by platformFindings, in the job platform of assets.yml alone.
      if (runner !== RUNNER && !(file === ASSETS && runner === MATRIX_RUNNER)) found.push(`${at}: runs on ${runner}, not on ${RUNNER}`);
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
  // A workflow whose every job calls another workflow names no runner of its own.
  const calls = lines.some((line) => /^ {4}uses: \.\/\.github\/workflows\//.test(line));
  if (file.includes('/workflows/') && runners === 0 && !calls) found.push(`${file}: names no runner`);
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

/** The jobs of a workflow in their order: the keys of its block `jobs:`, each with the lines of its mapping. */
function jobsOf(text: string): Map<string, string[]> {
  const lines = text.split('\n');
  const jobs = new Map<string, string[]>();
  let current: string[] | undefined;
  for (const line of lines.slice(lines.indexOf('jobs:') + 1)) {
    if (/^\S/.test(line) && !line.startsWith('#')) break;
    const name = /^ {2}([A-Za-z0-9_-]+): *$/.exec(line)?.[1];
    if (name !== undefined) {
      current = [];
      jobs.set(name, current);
    } else if (current !== undefined && !line.trim().startsWith('#')) current.push(line);
  }
  return jobs;
}

/**
 * The findings of the last job of the workflow ci: it is ci-passed, runs after a failed job, needs every other job and
 * judges the results of its needs with `make ci-passed FAIL_SKIPPED=1`.
 */
function lastJobFindings(file: string, text: string): string[] {
  const jobs = jobsOf(text);
  const names = [...jobs.keys()];
  const last = names.at(-1);
  if (last === undefined) return [`${file}: has no job`];
  if (last !== LAST) return [`${file}: the last job is ${last}, not ${LAST}`];
  const lines = jobs.get(LAST) ?? [];
  const value = (key: string): string | undefined => lines.map((line) => valueOf(line, key)).find((found) => found !== undefined);
  const found: string[] = [];
  const condition = value('if');
  if (condition !== '${{ always() }}') found.push(`${file}: the job ${LAST} has the condition ${condition ?? 'none'}, not \${{ always() }}`);
  const needs = /^\[(.*)\]$/.exec(value('needs') ?? '')?.[1]?.split(',').map((name) => name.trim()).filter((name) => name !== '') ?? [];
  const others = names.filter((name) => name !== LAST);
  if ([...needs].sort().join(' ') !== [...others].sort().join(' '))
    found.push(`${file}: the job ${LAST} needs ${needs.join(', ') || 'no job'}, not every other job: ${others.join(', ') || 'none'}`);
  if (!lines.some((line) => valueOf(line, 'run') === LAST_RUN)) found.push(`${file}: the job ${LAST} does not run ${LAST_RUN}`);
  return found;
}

/** The steps of the lines of a job: each step its lines, from its line `      - ` to the next. */
function stepsOf(lines: string[]): string[][] {
  const steps: string[][] = [];
  const start = lines.indexOf('    steps:');
  if (start < 0) return steps;
  for (const line of lines.slice(start + 1)) {
    if (/^ {0,4}\S/.test(line)) break;
    if (line.startsWith('      - ')) steps.push([]);
    steps.at(-1)?.push(line);
  }
  return steps;
}

/** The recipe lines of the target `target` of the text of a Makefile, without their tab; empty when no rule names it. */
function recipeOf(makefile: string, target: string): string[] {
  const lines = makefile.split('\n');
  const at = lines.findIndex((line) => new RegExp(`^${target.replaceAll('.', '\\.')} *:(?!=)(?! *[A-Za-z_]+ *[:?+]?=)`).test(line));
  if (at < 0) return [];
  const recipe: string[] = [];
  for (const line of lines.slice(at + 1)) {
    if (!line.startsWith('\t')) break;
    recipe.push(line.slice(1));
  }
  return recipe;
}

/**
 * The recipe of the job `job` of the workflow `file`: ci-<job> for a job of ci.yml, and ci-<workflow>-<job> for a job of a
 * workflow that ci.yml calls, assets.yml, whose job assets / <job> has the recipe ci-assets-<job>.
 */
function recipeName(file: string, job: string): string {
  return file === CI ? `ci-${job}` : `ci-${basename(file, '.yml')}-${job}`;
}

/**
 * The findings of the steps of the jobs of a workflow that CI runs: each job that has steps runs one make target, recipeName,
 * or for ci-passed its judgement LAST_RUN, with arguments of JOB_ARGUMENT alone; every other step is a uses step, a
 * condition stands on a uses step alone, and no job or step sets env but a job that sets GH_TOKEN alone (JOB_ENV). A job
 * that calls a workflow has no steps of its own.
 */
function jobStepFindings(file: string, text: string): string[] {
  const found: string[] = [];
  for (const [name, lines] of jobsOf(text)) {
    if (lines.some((line) => /^ {4}uses: /.test(line))) continue;
    const at = `${file}: the job ${name}`;
    const env = lines.indexOf(JOB_ENV[0] ?? '');
    const rest = env < 0 ? lines : [...lines.slice(0, env), ...lines.slice(env + 1).filter((line, index) => index !== 0 || line !== JOB_ENV[1])];
    if (env >= 0 && (lines[env + 1] !== JOB_ENV[1] || /^ {6}\S/.test(lines[env + 2] ?? ''))) found.push(`${at} sets env other than GH_TOKEN`);
    if (rest.some((line) => /^ +(?:- )?env:/.test(line))) found.push(`${at} sets env`);
    const steps = stepsOf(lines);
    const runs = steps.map((step) => step.map((line) => valueOf(line, 'run')).find((run) => run !== undefined)).filter((run) => run !== undefined);
    if (runs.length !== 1) found.push(`${at} has ${String(runs.length)} run steps, not one`);
    const [run] = runs;
    if (run !== undefined) {
      if (name === LAST) {
        if (run !== LAST_RUN) found.push(`${at} runs ${run}, not ${LAST_RUN}`);
      } else {
        const [make, target, ...rest] = run.match(/(?:"[^"]*"|\$\{\{[^}]*\}\}|[^\s"])+/g) ?? [];
        if (make !== 'make' || target !== recipeName(file, name)) found.push(`${at} runs ${run}, not make ${recipeName(file, name)}`);
        for (const argument of rest) if (!JOB_ARGUMENT.test(argument)) found.push(`${at} passes ${argument}, which is no make variable of a value, a matrix value or an input`);
      }
    }
    for (const step of steps) {
      const uses = step.some((line) => valueOf(line, 'uses') !== undefined);
      if (!uses && step.some((line) => /^ {6}(?:- | {2})if:/.test(line))) found.push(`${at} has a condition on a run step`);
    }
  }
  return found;
}


/** Accept only the tracked include-list convention, whose keys cannot alter native expansion. */
function includeMatrix(lines: string[], php: boolean): boolean {
  const matrixAt = lines.findIndex((line) => line.startsWith('      matrix:'));
  const matrixEnd = lines.findIndex((line, index) => index > matrixAt && /^ {0,6}\S/.test(line));
  const matrix = lines.slice(matrixAt, matrixEnd < 0 ? undefined : matrixEnd).filter((line) => line.trim() !== '' && !line.trim().startsWith('#')).join('\n');
  const includeOnly = php
    ? /^      matrix:\n        include:\n(?:          - platform: [a-z0-9-]+\n            runner: [a-z0-9.-]+\n            php: '[0-9]+\.[0-9]+'(?:\n|$))+$/
    : /^      matrix:\n        include:\n(?:          - platform: [a-z0-9-]+\n            runner: [a-z0-9.-]+(?:\n|$))+$/;
  return lines.filter((line) => line.startsWith('      matrix:')).length === 1 && includeOnly.test(matrix);
}

/**
 * The findings of the runners of assets.yml: the job platform runs on the runner of its matrix, whose entries are the
 * platforms of the release, each with its runner, and only the separate PHP job may also run on a runner of the matrix.
 */
function platformFindings(file: string, text: string): string[] {
  const found: string[] = [];
  const jobs = jobsOf(text);
  for (const [name, lines] of jobs) {
    if (name !== 'platform' && name !== 'php' && lines.some((line) => valueOf(line, 'runs-on') === MATRIX_RUNNER)) found.push(`${file}: the job ${name} runs on the runner of a matrix`);
  }
  const lines = jobs.get('platform');
  if (lines === undefined) return [...found, `${file}: has no job platform`];
  if (!lines.some((line) => valueOf(line, 'runs-on') === MATRIX_RUNNER)) found.push(`${file}: the job platform does not run on ${MATRIX_RUNNER}`);
  if (!includeMatrix(lines, false)) found.push(`${file}: the job platform matrix must be one explicit include list with only platform and runner`);
  const entries: Record<string, string> = {};
  let platform: string | undefined;
  let count = 0;
  for (const line of lines) {
    const named = valueOf(line, 'platform');
    if (named !== undefined && line.trim().startsWith('- ')) platform = named;
    const runner = valueOf(line, 'runner');
    if (runner !== undefined && platform !== undefined) {
      count += 1;
      entries[platform] = runner;
      platform = undefined;
    }
  }
  const byPlatform = (left: [string, string], right: [string, string]): number => left[0].localeCompare(right[0]);
  if (JSON.stringify(Object.entries(entries).sort(byPlatform)) !== JSON.stringify(Object.entries(PLATFORM_RUNNERS).sort(byPlatform)))
    found.push(`${file}: the job platform runs on ${JSON.stringify(entries)}, not on ${JSON.stringify(PLATFORM_RUNNERS)}`);
  if (count !== Object.keys(entries).length) found.push(`${file}: the job platform repeats a platform`);
  return found;
}

// The run step of the job php and the one line of its recipe ci-assets-php that builds the extension of the minor of the job.
const PHP_RUN = 'make ci-assets-php EXPECTED_PHP_MINOR=${{ matrix.php }}';
const PHP_BUILD = '$(MAKE) --no-print-directory release-php-ext PHP=php EXPECTED_PHP_MINOR=$(EXPECTED_PHP_MINOR) $(if $(LLVM_PATH),LLVM_ROOT=$(LLVM_PATH))';
// The target-specific lines that empty the library paths of the dynamic loader for ci-assets-php, which install-llvm-action
// sets with /usr/lib first, so that the PHP of Homebrew does not load the libraries of the system.
const PHP_LIBRARY_PATHS = ['ci-assets-php: DYLD_LIBRARY_PATH :=', 'ci-assets-php: LD_LIBRARY_PATH :='];
// The native assets of the job platform, which its recipe ci-assets-platform builds once each.
const NON_PHP = ['release-go-static', 'release-wheel', 'release-npm-platform'];

/** The count of the times that the recipe lines of `recipe` name the target `target` as a word. */
function naming(recipe: string[], target: string): number {
  return recipe.reduce((count, line) => count + (line.match(new RegExp(`(?<=^| )${target}(?= |$)`, 'g')) ?? []).length, 0);
}

/**
 * The PHP job isolates every platform and declared minor in a fresh runner: its one PHP setup owns the minor of the matrix
 * and precedes its run step, PHP_RUN, whose recipe ci-assets-php of `makefile` builds the extension once, with PHP_BUILD.
 */
function phpMinorFindings(file: string, text: string, minors: string[], makefile: string): string[] {
  const jobs = jobsOf(text);
  const lines = jobs.get('php');
  if (lines === undefined) return [`${file}: has no job php`];
  const found: string[] = [];
  const fail = (message: string): void => { found.push(`${file}: ${message}`); };
  if (!includeMatrix(lines, true)) fail('the job php matrix must be one explicit include list with only platform, runner and php');
  const entries: { platform: string; runner?: string; php?: string }[] = [];
  for (const line of lines) {
    const platform = valueOf(line, 'platform');
    if (platform !== undefined && line.trim().startsWith('- ')) entries.push({ platform });
    const current = entries.at(-1);
    const runner = valueOf(line, 'runner');
    const php = valueOf(line, 'php')?.replace(/^'|'$/g, '');
    if (current !== undefined && runner !== undefined) current.runner = runner;
    if (current !== undefined && php !== undefined) current.php = php;
  }
  const expected = Object.entries(PLATFORM_RUNNERS).flatMap(([platform, runner]) => minors.map((php) => ({ platform, runner, php })));
  const canonical = (values: typeof entries): string => values.map((entry) => JSON.stringify(entry)).sort().join(' ');
  if (canonical(entries) !== canonical(expected)) fail('the job php does not have the exact platform, runner and PHP minor product');
  if (lines.filter((line) => valueOf(line, 'runs-on') === MATRIX_RUNNER).length !== 1) fail('the job php must run on matrix.runner');
  const setups = lines.filter((line) => valueOf(line, 'uses')?.startsWith('shivammathur/setup-php@') === true);
  const versions = lines.map((line) => valueOf(line, 'php-version')).filter((value) => value !== undefined);
  if (setups.length !== 1 || versions.length !== 1 || versions[0] !== '${{ matrix.php }}') fail('the job php must set up exactly matrix.php once');
  const steps = stepsOf(lines);
  const setupAt = steps.findIndex((step) => step.some((line) => valueOf(line, 'uses')?.startsWith('shivammathur/setup-php@') === true));
  const buildAt = steps.findIndex((step) => step.some((line) => valueOf(line, 'run') !== undefined));
  const setup = steps[setupAt] ?? [];
  if (!setup.includes('        with:') || !setup.includes('          php-version: ${{ matrix.php }}') || setupAt >= buildAt) fail('the PHP setup step must own its minor input and precede the build');
  const runs = lines.map((line) => valueOf(line, 'run')).filter((value) => value !== undefined);
  if (runs.length !== 1 || runs[0] !== PHP_RUN) fail(`the job php must run ${PHP_RUN} once`);
  const upload = lines.findIndex((line) => valueOf(line, 'uses')?.startsWith('actions/upload-artifact@') === true);
  if (upload < 0 || !lines.slice(upload).some((line) => valueOf(line, 'name') === 'assets-php-${{ matrix.platform }}-${{ matrix.php }}')) fail('the job php must publish its unique platform and minor artifact');
  const recipe = (name: string): string[] => recipeOf(makefile, recipeName(file, name));
  const php = recipe('php');
  if (naming(php, 'release-php-ext') !== 1 || !php.includes(PHP_BUILD)) fail(`the recipe ci-assets-php must build once with ${PHP_BUILD}`);
  for (const line of PHP_LIBRARY_PATHS) if (!makefile.split('\n').includes(line)) fail(`the Makefile must hold ${line}`);
  for (const [name, other] of jobs) {
    if (name === 'php' || name === 'platform') continue;
    if (other.some((line) => valueOf(line, 'uses')?.startsWith('shivammathur/setup-php@') === true || valueOf(line, 'php-version') !== undefined) || ['release-php-ext', ...NON_PHP].some((target) => naming(recipe(name), target) > 0)) fail(`the job ${name} repeats a native asset setup or build`);
  }
  if (NON_PHP.some((target) => naming(php, target) > 0)) fail('the job php must not repeat non-PHP assets');
  const platform = jobs.get('platform') ?? [];
  if (platform.some((line) => valueOf(line, 'php-version') !== undefined) || naming(recipe('platform'), 'release-php-ext') > 0) fail('the job platform must not install or build PHP');
  for (const target of NON_PHP) {
    if (naming(recipe('platform'), target) !== 1) fail(`the job platform must build ${target} once`);
  }
  return found;
}

/** The findings of a workflow that must call assets.yml: one of its jobs uses it (binding GN-19i). */
function callFindings(file: string, text: string): string[] {
  const calls = [...jobsOf(text).values()].some((lines) => lines.some((line) => /^ {4}uses: \.\/\.github\/workflows\/assets\.yml$/.test(line)));
  return calls ? [] : [`${file}: no job calls ./${ASSETS}`];
}

// The targets that test the bindings of every target of the contract (binding BD-27), which the job bindings runs.
const BINDING_TESTS = ['lint-bindings', 'check-declarations', 'test-python', 'test-node', 'test-wasm', 'test-php', 'test-go'];
// The run step of the job bindings, the action that gives bay and its machine setup before it, and the target that its
// recipe ci-bindings runs before the tests: the Redis image of the service of the generated tests (SE-10.7), as the job
// test pulls it. The machine setup of bay is applied once per machine, so no recipe of a job applies it (MACHINE_SETUP).
const BINDINGS_RUN = 'make ci-bindings';
const BAY_ACTION = './.github/actions/bay';
// install-python-packages installs the lock of the Python root, with the wheel of php-serialization, into .venv (T1.59).
const BINDING_PREREQUISITES = ['install-python-packages', 'redis-image'];
const MACHINE_SETUP = /(?:^|\s)(?:setup-bay|install|sudo)(?:\s|$)/;

/**
 * The findings of the job bindings of the workflow ci: it uses BAY_ACTION before it runs BINDINGS_RUN, its recipe
 * ci-bindings runs the targets of BINDING_PREREQUISITES in their order and then the tests of CI_BINDINGS, the recipes
 * ci-bindings and ci-test apply no machine setup of bay, and CI_BINDINGS of the Makefile names each target of BINDING_TESTS.
 */
function bindingsJobFindings(file: string, text: string, makefile: string): string[] {
  const found: string[] = [];
  const lines = jobsOf(text).get('bindings');
  if (lines === undefined) found.push(`${file}: has no job bindings`);
  else {
    const run = lines.findIndex((line) => valueOf(line, 'run') === BINDINGS_RUN);
    const bay = lines.findIndex((line) => valueOf(line, 'uses') === BAY_ACTION);
    if (run < 0) found.push(`${file}: the job bindings does not run ${BINDINGS_RUN}`);
    else if (bay < 0 || bay > run) found.push(`${file}: the job bindings does not use ${BAY_ACTION} before ${BINDINGS_RUN}`);
  }
  for (const target of ['ci-bindings', 'ci-test']) {
    if (recipeOf(makefile, target).some((line) => MACHINE_SETUP.test(line))) found.push(`Makefile: the recipe ${target} applies the machine setup of bay`);
  }
  const recipe = recipeOf(makefile, 'ci-bindings');
  const order = [...BINDING_PREREQUISITES, '$(CI_BINDINGS)'];
  const at = order.map((target) => recipe.findIndex((line) => line.split(' ').includes(target)));
  if (at.some((index) => index < 0) || at.some((index, position) => position > 0 && index <= (at[position - 1] ?? -1)))
    found.push(`Makefile: the recipe ci-bindings does not run ${order.join(', ')} in this order`);
  const named = /^CI_BINDINGS := (.*)$/m.exec(makefile)?.[1]?.trim().split(/ +/) ?? [];
  const missing = BINDING_TESTS.filter((target) => !named.includes(target));
  if (missing.length > 0) found.push(`Makefile: CI_BINDINGS names no ${missing.join(', ')}`);
  return found;
}

const TARGETS = new Set(['docs-check', 'release-tag', 'ci-passed']);
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
  assert.deepEqual(broken('run: make docs-check', 'run: make docs-check TOOLS="uv python"'), []);
  assert.deepEqual(broken('run: make docs-check', 'run: make docs-check TOOLS="a; b"'), [`${CI}:16: the run step is not one make target: make docs-check TOOLS="a; b"`]);
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

const LAST_JOB = `  rust:
    runs-on: ubuntu-26.04-arm
    steps:
      - run: make docs-check
  ci-passed:
    if: \${{ always() }}
    needs: [shared, rust]
    runs-on: ubuntu-26.04-arm
    steps:
      - uses: ./.github/actions/bootstrap
      - name: every job passed
        run: make ci-passed FAIL_SKIPPED=1 RESULTS='\${{ toJSON(needs) }}'
`;

test('the last job of the workflow ci is ci-passed, which needs every other job and judges their results', () => {
  const good = GOOD + LAST_JOB;
  assert.deepEqual(findings(CI, good, TARGETS, ACTIONS), []);
  assert.deepEqual(lastJobFindings(CI, good), []);
  const broken = (from: string, to: string): string[] => {
    assert.ok(good.includes(from), `the sample holds no ${from}`);
    return lastJobFindings(CI, good.replace(from, to));
  };
  assert.deepEqual(lastJobFindings(CI, GOOD), [`${CI}: the last job is shared, not ci-passed`]);
  assert.deepEqual(lastJobFindings(CI, `${good}  later:\n    runs-on: ubuntu-26.04-arm\n`), [`${CI}: the last job is later, not ci-passed`]);
  assert.deepEqual(lastJobFindings(CI, 'name: ci\njobs:\n'), [`${CI}: has no job`]);
  assert.deepEqual(broken('    if: ${{ always() }}\n', ''), [`${CI}: the job ci-passed has the condition none, not \${{ always() }}`]);
  assert.deepEqual(broken('if: ${{ always() }}', 'if: ${{ success() }}'), [`${CI}: the job ci-passed has the condition \${{ success() }}, not \${{ always() }}`]);
  assert.deepEqual(broken('needs: [shared, rust]', 'needs: [shared]'), [`${CI}: the job ci-passed needs shared, not every other job: shared, rust`]);
  assert.deepEqual(broken('    needs: [shared, rust]\n', ''), [`${CI}: the job ci-passed needs no job, not every other job: shared, rust`]);
  assert.deepEqual(broken('needs: [shared, rust]', 'needs: [shared, rust, other]'), [`${CI}: the job ci-passed needs shared, rust, other, not every other job: shared, rust`]);
  assert.deepEqual(broken(" RESULTS='${{ toJSON(needs) }}'", ''), [`${CI}: the job ci-passed does not run ${LAST_RUN}`]);
  assert.deepEqual(broken('run: make ci-passed FAIL_SKIPPED=1', 'run: make ci-passed'), [`${CI}: the job ci-passed does not run ${LAST_RUN}`]);
});

test('the Makefile rules name their targets', () => {
  assert.deepEqual([...makeTargets('A := b\n.PHONY: x\nx y: z\n\trun: it\nbindings-%: w\ntest-php: export PHP := php\n')].sort(), ['test-php', 'x', 'y']);
});

const PLATFORM = `name: assets
jobs:
  platform:
    strategy:
      matrix:
        include:
          - platform: linux-x64-gnu
            runner: ubuntu-26.04
          - platform: linux-arm64-gnu
            runner: ubuntu-26.04-arm
          - platform: darwin-arm64
            runner: macos-15
    runs-on: \${{ matrix.runner }}
    steps:
      - run: make docs-check
  npm:
    runs-on: ubuntu-26.04-arm
    steps:
      - run: make docs-check
`;

test('the job platform of assets.yml alone runs on the runner of each platform of the release', () => {
  assert.deepEqual(findings(ASSETS, PLATFORM, TARGETS, ACTIONS), []);
  assert.deepEqual(platformFindings(ASSETS, PLATFORM), []);
  const broken = (from: string, to: string): string[] => {
    assert.ok(PLATFORM.includes(from), `the sample holds no ${from}`);
    const text = PLATFORM.replace(from, to);
    return [...findings(ASSETS, text, TARGETS, ACTIONS), ...platformFindings(ASSETS, text)];
  };
  // The runner of a matrix in another workflow is a runner that is not ubuntu-26.04-arm.
  assert.deepEqual(findings(CI, PLATFORM, TARGETS, ACTIONS), [`${CI}:13: runs on \${{ matrix.runner }}, not on ${RUNNER}`]);
  assert.deepEqual(broken('runner: ubuntu-26.04\n', 'runner: ubuntu-24.04\n'), [`${ASSETS}: the job platform runs on {"linux-x64-gnu":"ubuntu-24.04","linux-arm64-gnu":"ubuntu-26.04-arm","darwin-arm64":"macos-15"}, not on ${JSON.stringify(PLATFORM_RUNNERS)}`]);
  assert.deepEqual(broken('          - platform: darwin-arm64\n            runner: macos-15\n', ''), [`${ASSETS}: the job platform runs on {"linux-x64-gnu":"ubuntu-26.04","linux-arm64-gnu":"ubuntu-26.04-arm"}, not on ${JSON.stringify(PLATFORM_RUNNERS)}`]);
  assert.deepEqual(broken('    runs-on: ${{ matrix.runner }}', '    runs-on: macos-15'), [`${ASSETS}:13: runs on macos-15, not on ${RUNNER}`, `${ASSETS}: the job platform does not run on ${MATRIX_RUNNER}`]);
  assert.deepEqual(broken('  npm:\n    runs-on: ubuntu-26.04-arm', '  npm:\n    runs-on: ${{ matrix.runner }}'), [`${ASSETS}: the job npm runs on the runner of a matrix`]);
  assert.deepEqual(platformFindings(ASSETS, PLATFORM.replace('  platform:', '  build:')), [`${ASSETS}: the job build runs on the runner of a matrix`, `${ASSETS}: has no job platform`]);
});

test('a workflow that calls assets.yml has no finding of the call, and one that does not is a finding', () => {
  const calling = `${GOOD}  assets:\n    uses: ./.github/workflows/assets.yml\n`;
  assert.deepEqual(callFindings(CI, calling), []);
  assert.deepEqual(findings(CI, calling, TARGETS, new Set([...ACTIONS, `./${ASSETS}`])), []);
  assert.deepEqual(callFindings(CI, GOOD), [`${CI}: no job calls ./${ASSETS}`]);
});

const PHP_SAMPLE = `jobs:
  platform:
    runs-on: \${{ matrix.runner }}
    steps:
      - run: make ci-assets-platform
  php:
    strategy:
      matrix:
        include:
${Object.entries(PLATFORM_RUNNERS).flatMap(([platform, runner]) => ['8.4', '8.5'].map((php) => `          - platform: ${platform}\n            runner: ${runner}\n            php: '${php}'`)).join('\n')}
    runs-on: \${{ matrix.runner }}
    steps:
      - uses: shivammathur/setup-php@eb7c497e18156a6bbabfef1d3a82760b9eda3962
        with:
          php-version: \${{ matrix.php }}
      - run: make ci-assets-php EXPECTED_PHP_MINOR=\${{ matrix.php }}
      - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
        with:
          name: assets-php-\${{ matrix.platform }}-\${{ matrix.php }}
`;
const PHP_MAKEFILE = `ci-assets-platform: ## The assets of the platform
	$(MAKE) --no-print-directory release-recipes
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) release-go-static release-wheel release-npm-platform
ci-assets-php: DYLD_LIBRARY_PATH :=
ci-assets-php: LD_LIBRARY_PATH :=
ci-assets-php: ## The PHP extension of EXPECTED_PHP_MINOR
	$(MAKE) --no-print-directory release-recipes
	${PHP_BUILD}
ci-assets-other: ## Another job
	$(MAKE) --no-print-directory release-recipes
`;

test('the tracked PHP asset workflow isolates each platform and minor (T1.33-1)', () => {
  // T1.73: the PHP minors of the extension are those of phpExtension.minors of config/toolchain.json (binding GN-19a),
  // which release-platform-assets of binding reads when it requires one extension of each platform and minor; the
  // matrix of the job php builds exactly these.
  const minors = (JSON.parse(readFileSync(join(root, 'config/toolchain.json'), 'utf8')) as { phpExtension?: { minors?: string[] } }).phpExtension?.minors;
  assert.ok(Array.isArray(minors) && minors.length > 0, 'config/toolchain.json names no PHP minor of the extension in phpExtension.minors');
  assert.deepEqual(phpMinorFindings(ASSETS, readFileSync(join(root, ASSETS), 'utf8'), minors, readFileSync(join(root, 'Makefile'), 'utf8')), []);
});

test('PHP asset jobs isolate the exact platform and minor product (T1.33-1)', () => {
  assert.deepEqual(phpMinorFindings(ASSETS, PHP_SAMPLE, ['8.4', '8.5'], PHP_MAKEFILE), []);
  const mutations = [
    PHP_SAMPLE.replace("            php: '8.4'", "            php: '8.5'"),
    PHP_SAMPLE.replace(/          - platform: linux-x64-gnu\n            runner: ubuntu-26.04\n            php: '8.4'\n/, ''),
    PHP_SAMPLE.replaceAll('    runs-on: ${{ matrix.runner }}\n    steps:', "          - platform: linux-x64-gnu\n            runner: ubuntu-26.04\n            php: '8.4'\n    runs-on: ${{ matrix.runner }}\n    steps:"),
    PHP_SAMPLE.replace('            runner: ubuntu-26.04', '            runner: macos-15'),
    PHP_SAMPLE.replace("            php: '8.4'", "            php: '8.6'"),
    PHP_SAMPLE.replaceAll('    runs-on: ${{ matrix.runner }}\n    steps:', '    runs-on: macos-15\n    steps:'),
    PHP_SAMPLE.replace('php-version: ${{ matrix.php }}', "php-version: '8.4'"),
    PHP_SAMPLE.replace('      - run: make ci-assets-php', '      - uses: shivammathur/setup-php@duplicate\n      - run: make ci-assets-php'),
    PHP_SAMPLE.replace(' EXPECTED_PHP_MINOR=${{ matrix.php }}', ''),
    PHP_SAMPLE.replace('      - run: make ci-assets-php', '      - run: make ci-assets-php PHP=php\n      - run: make ci-assets-php'),
    PHP_SAMPLE.replace('assets-php-${{ matrix.platform }}-${{ matrix.php }}', 'assets-${{ matrix.platform }}'),
  ];
  for (const [index, mutation] of mutations.entries()) assert.notDeepEqual(phpMinorFindings(ASSETS, mutation, ['8.4', '8.5'], PHP_MAKEFILE), [], `mutation ${index}`);
  const recipes = [
    PHP_MAKEFILE.replace(' release-wheel ', ' '),
    PHP_MAKEFILE.replace(' release-wheel ', ' release-wheel release-wheel '),
    PHP_MAKEFILE.replace('ci-assets-platform: ## The assets of the platform\n', `ci-assets-platform: ## The assets of the platform\n\t${PHP_BUILD}\n`),
    PHP_MAKEFILE.replace(' EXPECTED_PHP_MINOR=$(EXPECTED_PHP_MINOR)', ''),
    PHP_MAKEFILE.replace(PHP_BUILD, `${PHP_BUILD}\n\t${PHP_BUILD}`),
    PHP_MAKEFILE.replace(PHP_BUILD, `${PHP_BUILD}\n\t$(MAKE) release-wheel`),
    PHP_MAKEFILE.replace('ci-assets-php: DYLD_LIBRARY_PATH :=\n', ''),
    PHP_MAKEFILE.replace('ci-assets-php: LD_LIBRARY_PATH :=\n', ''),
  ];
  for (const [index, recipe] of recipes.entries()) assert.notDeepEqual(phpMinorFindings(ASSETS, PHP_SAMPLE, ['8.4', '8.5'], recipe), [], `recipe mutation ${index}`);
  assert.notDeepEqual(phpMinorFindings(ASSETS, PHP_SAMPLE, ['8.4', '8.5', '8.6'], PHP_MAKEFILE), []);
});

test('PHP jobs reject matrix shape changes and detached or late setup (T1.33-1)', () => {
  const build = `      - run: ${PHP_RUN}\n`;
  const version = '          php-version: ${{ matrix.php }}\n';
  const mutations = [
    PHP_SAMPLE.replace('      matrix:\n', '      matrix:\n        extra: [one, two]\n'),
    PHP_SAMPLE.replace('      matrix:\n', '      matrix:\n        exclude: [{platform: linux-x64-gnu}]\n'),
    PHP_SAMPLE.replace('      matrix:\n', '      matrix: {extra: [one, two]}\n'),
    PHP_SAMPLE.replace('        include:\n', '        include: []\n'),
    PHP_SAMPLE.replace('        include:\n', '        include:\n        include:\n'),
    PHP_SAMPLE.replace("            php: '8.4'", "            php: '8.4'\n            extra: one"),
    PHP_SAMPLE.replace(build, '').replace('      - uses: shivammathur/setup-php@', build + '      - uses: shivammathur/setup-php@'),
    PHP_SAMPLE.replace(version, '').replace(build, build + '        with:\n' + version),
  ];
  for (const [index, mutation] of mutations.entries()) assert.notDeepEqual(phpMinorFindings(ASSETS, mutation, ['8.4', '8.5'], PHP_MAKEFILE), [], `shape/setup mutation ${index}`);
});

test('asset ownership rejects non-PHP matrix axes and builds in other jobs (T1.33-1)', () => {
  for (const extra of ['        extra: [one, two]\n', '        exclude: [{platform: linux-x64-gnu}]\n']) {
    assert.notDeepEqual(platformFindings(ASSETS, PLATFORM.replace('      matrix:\n', '      matrix:\n' + extra)), []);
  }
  const other = PHP_SAMPLE + '  other:\n    runs-on: ubuntu-26.04-arm\n    steps:\n      - run: make ci-assets-other\n';
  assert.deepEqual(phpMinorFindings(ASSETS, other, ['8.4', '8.5'], PHP_MAKEFILE), []);
  assert.notDeepEqual(phpMinorFindings(ASSETS, other.replace('      - run: make ci-assets-other', '      - uses: shivammathur/setup-php@duplicate\n      - run: make ci-assets-other'), ['8.4', '8.5'], PHP_MAKEFILE), []);
  for (const target of ['release-php-ext PHP=php', 'release-go-static', 'release-wheel', 'release-npm-platform']) {
    assert.notDeepEqual(phpMinorFindings(ASSETS, other, ['8.4', '8.5'], `${PHP_MAKEFILE}\t$(MAKE) ${target}\n`), [], target);
  }
});

const JOBS = `jobs:
  shared:
    runs-on: ubuntu-26.04-arm
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
      - uses: ./.github/actions/bootstrap
      - name: shared checks
        run: make ci-shared
      - name: report
        if: \${{ !cancelled() }}
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
  php:
    runs-on: \${{ matrix.runner }}
    steps:
      - run: make ci-php EXPECTED_PHP_MINOR=\${{ matrix.php }} RELEASE=\${{ inputs.release }} TOOLS="uv python"
  assets:
    uses: ./.github/workflows/assets.yml
${LAST_JOB.replace(/^ {2}rust:\n(?: {4}.*\n)*/, '')}`;

test('every job of CI runs its recipe ci-<job> in one run step, and its other steps are uses steps', () => {
  assert.deepEqual(jobStepFindings(CI, JOBS), []);
  assert.deepEqual(jobStepFindings(ASSETS, JOBS.replace('make ci-php ', 'make ci-assets-php ').replace(/^ {2}(?:shared|assets|ci-passed):\n(?: {4}.*\n)*/gm, '')), []);
  assert.deepEqual(jobStepFindings(ASSETS, JOBS.replace(/^ {2}(?:shared|assets|ci-passed):\n(?: {4}.*\n)*/gm, '')), [`${ASSETS}: the job php runs make ci-php EXPECTED_PHP_MINOR=\${{ matrix.php }} RELEASE=\${{ inputs.release }} TOOLS="uv python", not make ci-assets-php`]);
  const broken = (from: string, to: string): string[] => {
    assert.ok(JOBS.includes(from), `the sample holds no ${from}`);
    return jobStepFindings(CI, JOBS.replace(from, to));
  };
  assert.deepEqual(broken('        run: make ci-shared\n', ''), [`${CI}: the job shared has 0 run steps, not one`]);
  assert.deepEqual(broken('        run: make ci-shared\n', '        run: make ci-shared\n      - run: make install-tools\n'), [`${CI}: the job shared has 2 run steps, not one`]);
  assert.deepEqual(broken('run: make ci-shared', 'run: make shared-checks'), [`${CI}: the job shared runs make shared-checks, not make ci-shared`]);
  assert.deepEqual(broken('run: make ci-shared', 'run: make ci-shared CI_REPORT=${{ runner.temp }}'), [`${CI}: the job shared passes CI_REPORT=\${{ runner.temp }}, which is no make variable of a value, a matrix value or an input`]);
  assert.deepEqual(broken('run: make ci-shared', 'run: make ci-shared ${{ matrix.php }}'), [`${CI}: the job shared passes \${{ matrix.php }}, which is no make variable of a value, a matrix value or an input`]);
  assert.deepEqual(broken('      - name: shared checks\n', '      - name: shared checks\n        if: ${{ !cancelled() }}\n'), [`${CI}: the job shared has a condition on a run step`]);
  assert.deepEqual(broken('      - name: shared checks\n', '      - name: shared checks\n        env:\n          TOOLS: alint\n'), [`${CI}: the job shared sets env`]);
  assert.deepEqual(broken("RESULTS='${{ toJSON(needs) }}'", 'RESULTS=${{ toJSON(needs) }}'), [`${CI}: the job ci-passed runs make ci-passed FAIL_SKIPPED=1 RESULTS=\${{ toJSON(needs) }}, not ${LAST_RUN}`]);
  // The jobs of release.yml run ci-release-<job>, with the outputs of the job tag and quoted expressions of the event, and
  // set the token of the run alone.
  const release = `jobs:
  tag:
    runs-on: ubuntu-26.04-arm
    env:
      GH_TOKEN: \${{ github.token }}
    steps:
      - id: tag
        run: make ci-release-tag RUN="\${{ inputs.run || github.event.workflow_run.id }}" AUTO="\${{ github.event_name == 'workflow_run' && '1' || '' }}"
  release:
    runs-on: ubuntu-26.04-arm
    env:
      GH_TOKEN: \${{ github.token }}
    steps:
      - uses: ./.github/actions/bootstrap
      - run: make ci-release-release TAG=\${{ needs.tag.outputs.tag }} VERSION=\${{ needs.tag.outputs.version }}
`;
  assert.deepEqual(jobStepFindings(RELEASE, release), []);
  assert.deepEqual(jobStepFindings(RELEASE, release.replace('make ci-release-tag', 'make release-tag')), [`${RELEASE}: the job tag runs make release-tag RUN="\${{ inputs.run || github.event.workflow_run.id }}" AUTO="\${{ github.event_name == 'workflow_run' && '1' || '' }}", not make ci-release-tag`]);
  assert.deepEqual(jobStepFindings(RELEASE, release.replace('      GH_TOKEN: ${{ github.token }}\n    steps:\n      - id', '      GH_TOKEN: ${{ github.token }}\n      TAG: v1\n    steps:\n      - id')), [`${RELEASE}: the job tag sets env other than GH_TOKEN`]);
  assert.deepEqual(jobStepFindings(RELEASE, release.replace('      GH_TOKEN: ${{ github.token }}\n    steps:\n      - id', '      TAG: v1\n    steps:\n      - id')), [`${RELEASE}: the job tag sets env other than GH_TOKEN`]);
  assert.deepEqual(jobStepFindings(RELEASE, release.replace('      - run: make ci-release-release', '      - run: make release-tools\n      - run: make ci-release-release')), [`${RELEASE}: the job release has 2 run steps, not one`, `${RELEASE}: the job release runs make release-tools, not make ci-release-release`]);
  assert.deepEqual(jobStepFindings(RELEASE, release.replace('      - run: make ci-release-release', '      - env:\n          GIT_AUTHOR_NAME: x\n        run: make ci-release-release')), [`${RELEASE}: the job release sets env`]);
});

test('the job bindings runs make ci-bindings after the action bay, and no recipe applies the machine setup of bay', () => {
  const job = `${GOOD}  bindings:\n    runs-on: ubuntu-26.04-arm\n    steps:\n      - uses: ${BAY_ACTION}\n      - run: ${BINDINGS_RUN}\n`;
  const recipe = 'ci-bindings: ## The job bindings\n\t$(MAKE) install-rust\n\t$(MAKE) install-python-packages\n\t$(MAKE) redis-image\n\t$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_BINDINGS)\n';
  const makefile = `CI_BINDINGS := ${BINDING_TESTS.join(' ')}\n${recipe}ci-test:\n\t$(MAKE) redis-image\n\t$(MAKE) test\n`;
  const order = 'Makefile: the recipe ci-bindings does not run install-python-packages, redis-image, $(CI_BINDINGS) in this order';
  assert.deepEqual(bindingsJobFindings(CI, job, makefile), []);
  assert.deepEqual(bindingsJobFindings(CI, GOOD, makefile), [`${CI}: has no job bindings`]);
  assert.deepEqual(bindingsJobFindings(CI, job.replace(BINDINGS_RUN, 'make ci-bindings BAY_SUDO=1'), makefile), [`${CI}: the job bindings does not run ${BINDINGS_RUN}`]);
  assert.deepEqual(bindingsJobFindings(CI, job.replace(`      - uses: ${BAY_ACTION}\n`, ''), makefile), [`${CI}: the job bindings does not use ${BAY_ACTION} before ${BINDINGS_RUN}`]);
  assert.deepEqual(bindingsJobFindings(CI, job.replace(`      - uses: ${BAY_ACTION}\n      - run: ${BINDINGS_RUN}\n`, `      - run: ${BINDINGS_RUN}\n      - uses: ${BAY_ACTION}\n`), makefile), [`${CI}: the job bindings does not use ${BAY_ACTION} before ${BINDINGS_RUN}`]);
  assert.deepEqual(bindingsJobFindings(CI, job, makefile.replace('\t$(MAKE) redis-image\n\t$(NODE)', '\t$(NODE)')), [order]);
  assert.deepEqual(bindingsJobFindings(CI, job, makefile.replace('\t$(MAKE) redis-image\n\t$(NODE)', '\t$(NODE)').replace('$(CI_BINDINGS)\n', '$(CI_BINDINGS)\n\t$(MAKE) redis-image\n')), [order]);
  assert.deepEqual(bindingsJobFindings(CI, job, makefile.replace('\t$(MAKE) install-rust\n', '\t$(MAKE) setup-bay\n')), ['Makefile: the recipe ci-bindings applies the machine setup of bay']);
  assert.deepEqual(bindingsJobFindings(CI, job, makefile.replace('\t$(MAKE) test\n', '\t$(MAKE) test\n\tsudo -n bay install\n')), ['Makefile: the recipe ci-test applies the machine setup of bay']);
  assert.deepEqual(bindingsJobFindings(CI, job, makefile.replace(' test-php', '')), ['Makefile: CI_BINDINGS names no test-php']);
  assert.deepEqual(bindingsJobFindings(CI, job, recipe), [`Makefile: CI_BINDINGS names no ${BINDING_TESTS.join(', ')}`]);
});

/**
 * The targets of the release recipes that binding release-recipes writes for the contract, which the Makefile includes
 * (binding GN-19h). The file is generated and not tracked, so the test writes it into a folder of its own with the binding
 * of package.json and reads the targets from there.
 */
function releaseRecipeTargets(t: { after: (fn: () => void) => void }): Set<string> {
  const out = mkdtempSync(join(tmpdir(), 'session-store-recipes-'));
  t.after(() => { rmSync(out, { recursive: true, force: true }); });
  // binding runs on node, as BINDING of the Makefile runs it: a linked checkout gives its bin no mode of execution.
  execFileSync(process.execPath, [join(root, 'node_modules/@polyspec/binding/dist/cli.js'), 'release-recipes', '--root', root, '--out', out], { stdio: ['ignore', 'ignore', 'inherit'] });
  return makeTargets(readFileSync(join(out, 'packages/release.mk'), 'utf8'));
}

test('the workflows and the actions of the repository keep the rules', (t) => {
  const tracked = execFileSync('git', ['-C', root, 'ls-files', '-z', '--', '.github'], { encoding: 'utf8' }).split('\0').filter((file) => file !== '');
  const workflows = tracked.filter((file) => /^\.github\/workflows\/[^/]+\.yml$/.test(file));
  const actionFiles = tracked.filter((file) => /^\.github\/actions\/[^/]+\/action\.yml$/.test(file));
  assert.deepEqual(tracked.filter((file) => !workflows.includes(file) && !actionFiles.includes(file)), [], 'a tracked file of .github is no workflow and no action');
  for (const file of [CI, RELEASE, ASSETS]) assert.ok(workflows.includes(file), `${file} is not tracked`);
  const recipes = releaseRecipeTargets(t);
  assert.ok(recipes.has('release-go-static'), `the release recipes define no target release-go-static: ${[...recipes].join(', ')}`);
  const targets = new Set([...makeTargets(readFileSync(join(root, 'Makefile'), 'utf8')), ...recipes]);
  const actions = new Set([...actionFiles.map((file) => `./${dirname(file)}`), `./${ASSETS}`]);
  const found = [...workflows, ...actionFiles].flatMap((file) => findings(file, readFileSync(join(root, file), 'utf8'), targets, actions));
  found.push(...triggerFindings(CI, readFileSync(join(root, CI), 'utf8')));
  found.push(...lastJobFindings(CI, readFileSync(join(root, CI), 'utf8')));
  found.push(...platformFindings(ASSETS, readFileSync(join(root, ASSETS), 'utf8')));
  found.push(...phpMinorFindings(ASSETS, readFileSync(join(root, ASSETS), 'utf8'), ['8.4', '8.5'], readFileSync(join(root, 'Makefile'), 'utf8')));
  for (const file of [CI, ASSETS, RELEASE]) found.push(...jobStepFindings(file, readFileSync(join(root, file), 'utf8')));
  for (const file of [CI, RELEASE]) found.push(...callFindings(file, readFileSync(join(root, file), 'utf8')));
  found.push(...bindingsJobFindings(CI, readFileSync(join(root, CI), 'utf8'), readFileSync(join(root, 'Makefile'), 'utf8')));
  assert.deepEqual(found, []);
});

test('each job of assets.yml installs go before the release check of binding, which it runs in CI as in a release', () => {
  // T1.70: the release check of binding (release-binding-check, CK-70) checks the go.sum that binding scaffold writes
  // (CK-19) and starts go for it, and a job declares the tools that it runs (AGENTS.md): the release 0.0.2 failed after
  // its tag on the macOS runners with "go mod download did not start: spawnSync go ENOENT", because no job of assets.yml
  // installed go and the jobs ran the check only in a release. The dry run of each recipe with the arguments of its run
  // step, which the run of ci and a release give alike, runs the check, after an install-tools that names go.
  const make = process.env.MAKE ?? 'make';
  const environment = { ...process.env, MAKEFLAGS: '' };
  execFileSync(make, ['-s', '--no-print-directory', '-C', root, 'release-recipes'], { env: environment, stdio: ['ignore', 'ignore', 'inherit'] });
  const runs = [...readFileSync(join(root, ASSETS), 'utf8').matchAll(/^ +(?:- )?run: make (ci-assets-[a-z]+)(.*)$/gm)];
  assert.deepEqual(runs.map((run) => run[1]).sort(), ['ci-assets-npm', 'ci-assets-php', 'ci-assets-platform']);
  const findings: string[] = [];
  for (const [, recipe, given] of runs) {
    const values = given.replaceAll('${{ matrix.php }}', '8.4').trim();
    assert.doesNotMatch(values, /\$\{\{/, `an expression of ${recipe} that the test gives no value`);
    const lines = execFileSync(make, ['-n', '--no-print-directory', '-C', root, recipe, ...(values === '' ? [] : values.split(' '))], { encoding: 'utf8', env: environment }).split('\n');
    // The check is the line of binding, or the line of the sub-make of its target where the dry run does not enter it.
    const check = lines.findIndex((line) => / check \S+interface\.json --release |make\S* --no-print-directory release-binding-check$/.test(line));
    const go = lines.findIndex((line) => /install-tools\.mjs --no-self-update(?: \S+)* go(?: |$)/.test(line));
    if (check === -1) findings.push(`${recipe} does not run the release check of binding`);
    else if (go === -1 || go > check) findings.push(`${recipe} runs the release check of binding without go installed before it`);
  }
  assert.deepEqual(findings, []);
});

test('the recipe of the job release adds the downloaded assets of each platform with their folders', () => {
  // T1.72: the job release downloads the artifacts of the job assets into var/release/downloaded, where each asset lies
  // in the folder of its kind (go/<platform>, npm, php-ext, wheel), in which the recipes of binding read it from
  // var/release/assets. The line of ci-release-release that adds them runs here on such a tree: the release run of 0.0.3
  // failed at it with "cp: -r not specified; omitting directory".
  const recipe = readFileSync(join(root, 'Makefile'), 'utf8').split('\nci-release-release:\n')[1]?.split('\n\n')[0] ?? '';
  const lines = recipe.split('\n').filter((line) => line.includes('var/release/downloaded'));
  assert.equal(lines.length, 1, 'one line of ci-release-release adds the downloaded assets');
  const directory = mkdtempSync(join(tmpdir(), 'downloaded-'));
  const files = ['go/linux_arm64/libpolyspec_session_store_go.a', 'go/linux_arm64_musl/native-static-libs', 'npm/a.tgz', 'php-ext/a.zip', 'wheel/a.whl'];
  for (const file of files) {
    mkdirSync(dirname(join(directory, 'var/release/downloaded', file)), { recursive: true });
    writeFileSync(join(directory, 'var/release/downloaded', file), file);
  }
  // The archives of release-assets are in var/release/assets before the line runs, and stay.
  mkdirSync(join(directory, 'var/release/assets'), { recursive: true });
  writeFileSync(join(directory, 'var/release/assets/archive.tgz'), 'archive');
  execFileSync('sh', ['-ec', lines[0].trim()], { cwd: directory, stdio: ['ignore', 'ignore', 'inherit'] });
  for (const file of [...files, 'archive.tgz']) {
    assert.equal(existsSync(join(directory, 'var/release/assets', file)), true, `var/release/assets lacks ${file}`);
  }
  rmSync(directory, { recursive: true });
});

test('release-platform-assets names the asset of each platform and PHP minor of a tree of the job release', () => {
  // T1.74: the recipe of binding that uploads the assets of a release (release-platform-assets, GN-19g) runs here to its
  // gh, a stand-in that records its arguments, on a tree of the names that the jobs of assets.yml give their assets, for
  // the platforms of the job platform and the PHP minors of config/toolchain.json. It reads phpExtension.minors, without
  // which it fails; no run of ci reaches it, and the release run of 0.0.3 stopped one line before it.
  const make = process.env.MAKE ?? 'make';
  const directory = mkdtempSync(join(tmpdir(), 'platform-assets-'));
  const assets = join(directory, 'assets');
  const version = '9.9.9';
  const minors = (JSON.parse(readFileSync(join(root, 'config/toolchain.json'), 'utf8')) as { phpExtension: { minors: string[] } }).phpExtension.minors;
  const files = [
    ...['manylinux_2_34_x86_64', 'manylinux_2_34_aarch64', 'macosx_11_0_arm64'].map((tag) => `wheel/polyspec_session_store-${version}-cp314-abi3-${tag}.whl`),
    ...['', '-linux-x64-gnu', '-linux-arm64-gnu', '-darwin-arm64'].map((platform) => `npm/polyspec-session-store${platform}-${version}.tgz`),
    ...minors.flatMap((minor) => ['x86_64-linux-glibc', 'arm64-linux-glibc', 'arm64-darwin-bsdlibc'].map((platform) => `php-ext/php_polyspec_session_store-v${version}_php${minor}-${platform}-nts.zip`)),
  ];
  for (const file of files) {
    mkdirSync(dirname(join(assets, file)), { recursive: true });
    writeFileSync(join(assets, file), file);
  }
  const bin = join(directory, 'bin');
  mkdirSync(bin);
  writeFileSync(join(bin, 'gh'), `#!/bin/sh\nprintf '%s\\n' "$@" > "${join(directory, 'gh.arguments')}"\n`, { mode: 0o755 });
  const environment = { ...process.env, MAKEFLAGS: '', PATH: `${bin}:${process.env.PATH ?? ''}`, TAG: `v${version}` };
  execFileSync(make, ['-s', '--no-print-directory', '-C', root, 'release-recipes'], { env: environment, stdio: ['ignore', 'ignore', 'inherit'] });
  execFileSync(make, ['--no-print-directory', '-C', root, 'release-platform-assets', `RELEASE_ASSETS=${assets}`], { env: environment, stdio: ['ignore', 'ignore', 'inherit'] });
  const given = readFileSync(join(directory, 'gh.arguments'), 'utf8').trim().split('\n');
  assert.deepEqual(given.slice(0, 3), ['release', 'upload', `v${version}`]);
  const labels = given.slice(3).map((argument) => argument.split('#')[1]).sort();
  const platforms = ['darwin-arm64', 'linux-arm64-gnu', 'linux-x64-gnu'];
  assert.deepEqual(labels, [
    ...minors.flatMap((minor) => platforms.map((platform) => `polyspec-session-store-php-ext-${version}-php${minor}-${platform}`)),
    ...platforms.map((platform) => `polyspec-session-store-python-${version}-${platform}`),
    `polyspec-session-store-ts-${version}`,
    ...platforms.map((platform) => `polyspec-session-store-ts-${version}-${platform}`),
  ].sort());
  rmSync(directory, { recursive: true });
});

test('the job shared of ci builds the archives of a release from the commit of the run, with no tag', () => {
  // T1.78: the release makes and pushes its tag before it builds the archive of each package, so a failure of those
  // steps first shows after the tag. release-candidate-assets runs them on the committed HEAD for the version of the
  // first section of the change log (kit K1286), and the job shared runs it in every run of ci, after it installed the
  // uv with which kit reads the environment of the Python project.
  const makefile = readFileSync(join(root, 'Makefile'), 'utf8');
  assert.match(makefile, /^release-candidate-assets:\n\tRELEASE_ASSETS=var\/release-candidate\/assets \$\(NODE\) \$\(KIT\)\/release\.mjs candidate-assets$/m, 'release-candidate-assets does not build the candidate of the change log');
  assert.match(/^CI_SHARED := (.*)$/m.exec(makefile)?.[1] ?? '', /(^| )release-candidate-assets( |$)/, 'the job shared does not run release-candidate-assets');
  const make = process.env.MAKE ?? 'make';
  const lines = execFileSync(make, ['-n', '--no-print-directory', '-C', root, 'ci-shared'], { encoding: 'utf8', env: { ...process.env, MAKEFLAGS: '' } }).split('\n');
  const uv = lines.findIndex((line) => /install-tools\.mjs --no-self-update(?: \S+)* uv(?: |$)/.test(line));
  const targets = lines.findIndex((line) => /ci-targets\.mjs .* release-candidate-assets( |$)/.test(line));
  assert.ok(uv !== -1 && targets !== -1 && uv < targets, 'the job shared does not install uv before its targets');
});

test('the job release pushes the tags of the Go modules after each release-go-module, once', () => {
  // T1.88: binding 0.0.21 makes release-go-module write its tag into go-module-refs and pushes nothing; all tags go up in
  // one `git push --atomic` by release-go-module-push. ci-release-release that runs release-go-module without it leaves
  // the tag of the Go module of a release unpushed: json 0.0.37 lost its tag so. The dry run of the recipe with the
  // arguments of the release job shows the order of the two.
  const make = process.env.MAKE ?? 'make';
  const lines = execFileSync(make, ['-n', '--no-print-directory', '-C', root, 'ci-release-release', 'TAG=v0.0.7'], {
    encoding: 'utf8',
    env: { ...process.env, MAKEFLAGS: '', GITHUB_ACTOR: 'dry-run' },
  }).split('\n');
  // The line of a sub-make names its target after --no-print-directory; the echo of release-go-module names the push too.
  const module = lines.findIndex((line) => /--no-print-directory release-go-module$/.test(line));
  const push = lines.findIndex((line) => /--no-print-directory release-go-module-push/.test(line));
  assert.ok(module !== -1, 'ci-release-release does not run release-go-module');
  assert.equal(lines.filter((line) => /--no-print-directory release-go-module-push/.test(line)).length, 1, 'release-go-module-push runs once');
  assert.ok(push > module, 'release-go-module-push does not follow release-go-module');
});

test('the job release rehearses the Go module of its tag before release-go-module, once', () => {
  // T1.91: binding 0.0.21 proves the Go module of a tag by a consumer that the rehearsal (B2.539) runs on HEAD before the
  // tag exists; release-go-module then makes the tag. The dry run of ci-release-release names the rehearsal once, before it.
  const make = process.env.MAKE ?? 'make';
  const lines = execFileSync(make, ['-n', '--no-print-directory', '-C', root, 'ci-release-release', 'TAG=v0.0.8'], {
    encoding: 'utf8',
    env: { ...process.env, MAKEFLAGS: '', GITHUB_ACTOR: 'dry-run' },
  }).split('\n');
  const rehearse = lines.filter((line) => /--no-print-directory rehearse-go-module$/.test(line));
  const module = lines.findIndex((line) => /--no-print-directory release-go-module$/.test(line));
  assert.equal(rehearse.length, 1, 'rehearse-go-module runs once in ci-release-release');
  assert.ok(lines.indexOf(rehearse[0]) < module, 'the rehearsal does not precede release-go-module');
});

test('the workflow ci has the job release-rehearsal, which ci-passed needs, and runs make ci-release-rehearsal', () => {
  // T1.92: the rehearsal of the Go module runs on main push CI before the tag exists, so a release is gated by it (ci-passed).
  // The file is read only; no make target that calls binding runs here.
  const ci = readFileSync(join(root, '.github/workflows/ci.yml'), 'utf8');
  const job = ci.match(/^ {2}release-rehearsal:\n([\s\S]*?)(?=^ {2}\S|$(?![\s\S]))/m);
  assert.ok(job, 'the job release-rehearsal exists in ci.yml');
  assert.match(job[1], /run: make ci-release-rehearsal\b/, 'the job runs make ci-release-rehearsal');
  const passed = ci.match(/^ {2}ci-passed:\n[\s\S]*?needs: \[([^\]]*)\]/m);
  assert.ok(passed && passed[1].split(',').map((s) => s.trim()).includes('release-rehearsal'), 'ci-passed needs release-rehearsal');
});

// The rehearsal version of ci-release-rehearsal, read by scripts/rehearsal-version.mts from a change log of its own (T1.92).
function rehearsalVersion(changelog: string): { status: number; out: string; err: string } {
  const dir = mkdtempSync(join(tmpdir(), 'rehearsal-version-'));
  const file = join(dir, 'CHANGELOG.md');
  writeFileSync(file, changelog);
  try {
    const out = execFileSync(process.execPath, [join(root, 'scripts/rehearsal-version.mts'), file], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { status: 0, out, err: '' };
  } catch (error) {
    const failed = error as { status: number; stderr: string };
    return { status: failed.status, out: '', err: failed.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('the rehearsal version is the first ## X.Y.Z section after ## Unreleased (T1.92)', () => {
  const result = rehearsalVersion('# Change log\n\n## Unreleased\n\n- a change\n\n## 0.1.0\n\n## 0.0.9\n');
  assert.equal(result.status, 0, result.err);
  assert.equal(result.out, '0.1.0\n');
});

test('a section whose title has text after the version fails the rehearsal version (T1.92)', () => {
  const result = rehearsalVersion('## 0.0.7 extra\n');
  assert.notEqual(result.status, 0, 'the rehearsal version is refused');
  assert.match(result.err, /0\.0\.7 extra/);
});

test('a change log with no version section fails the rehearsal version (T1.92)', () => {
  const result = rehearsalVersion('# Change log\n\n## Unreleased\n');
  assert.notEqual(result.status, 0, 'the rehearsal version is refused');
  assert.match(result.err, /no ## X\.Y\.Z/);
});

test('the target npm-lock writes package-lock.json with the pinned npm and no other tool (T1.97)', () => {
  // The lock of the repository is written inside make by the npm of the checkout (the pinned tool), never by a command outside make.
  const makefile = readFileSync(join(root, 'Makefile'), 'utf8');
  const recipe = makefile.match(/^npm-lock:[^\n]*\n((?:\t[^\n]*\n?)+)/m);
  assert.ok(recipe, 'the target npm-lock exists in the Makefile');
  assert.match(recipe[1], /\$\(ONLINE\) \$\(NPM\) install --package-lock-only\b/, 'npm-lock runs the pinned npm with --package-lock-only');
});

test('the target python-lock writes uv.lock with the pinned uv of mise.lock, and no other tool (T1.95)', () => {
  // The lock of Python is written inside make by the uv of the checkout, as npm-lock writes the npm lock (T1.97).
  const makefile = readFileSync(join(root, 'Makefile'), 'utf8');
  const recipe = makefile.match(/^python-lock:[^\n]*\n((?:\t[^\n]*\n?)+)/m);
  assert.ok(recipe, 'the target python-lock exists in the Makefile');
  assert.match(recipe[1], /\$\(ONLINE\) \$\(UV\) lock\b/, 'python-lock runs the pinned uv with lock');
});
