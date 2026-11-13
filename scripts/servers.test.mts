// The commands of scripts/servers.sh and the run of scripts/test-run.sh (SE-10.4): bay starts and removes the servers
// of the tests, gives their names and addresses, and pulls and finds the image. The test runs the scripts with a
// stand-in of bay that it writes into a directory of its own, the only program of PATH besides /usr/bin and /bin, which
// records its arguments, so it needs neither bay nor a container runtime.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';

const scripts = dirname(fileURLToPath(import.meta.url));
const servers = join(scripts, 'servers.sh');
const testRun = join(scripts, 'test-run.sh');
const IMAGE = 'docker.io/library/redis:1.2.3@sha256:00';
const FILE = '/run/p/compose.yaml';

/** What the stand-in of one test does. */
interface Machine {
  /** Whether PATH holds bay. */
  bay: boolean;
  /** The exit status of the stand-in bay for each command that is named; 0 for the others. */
  status?: Record<string, number>;
}

/** The directory of the stand-in, with the commands that it received. */
interface Stand {
  directory: string;
  env: Record<string, string>;
  /** The commands that bay received so far, each as its words. */
  calls: () => string[][];
}

/** Writes the stand-in of `machine` into a directory that the test removes. */
function stand(t: TestContext, machine: Machine): Stand {
  const directory = mkdtempSync(join(tmpdir(), 'session-store-servers-'));
  t.after(() => { rmSync(directory, { recursive: true, force: true }); });
  const bin = join(directory, 'bin');
  mkdirSync(bin);
  const calls = join(directory, 'calls');
  if (machine.bay) {
    // Each command is one line of the file calls, its words joined by a tab. The command of bay is its first word
    // after the option -f and its value. pull keeps the Compose file that it received, exec keeps its standard input,
    // and the commands that print JSON print a document that names the command.
    const cases = Object.entries(machine.status ?? {}).map(([command, status]) => `${command}) echo "bay: ${command} fails" >&2; exit ${status} ;;`).join('\n');
    writeFileSync(join(bin, 'bay'), [
      '#!/bin/sh',
      'line=bay',
      'for word in "$@"; do line="$line\t$word"; done',
      `printf '%s\\n' "$line" >> '${calls}'`,
      'file=',
      'if [ "$1" = -f ]; then file=$2; shift 2; fi',
      'case "$1" in',
      cases,
      `pull) cat "$file" > '${join(directory, 'pulled')}' ;;`,
      `exec) cat > '${join(directory, 'input')}' ;;`,
      `plan | up) echo "{\\"command\\":\\"$1\\"}" ;;`,
      `check) echo 'ok       dns agent' ;;`,
      'esac',
      'exit 0',
      '',
    ].join('\n'));
    chmodSync(join(bin, 'bay'), 0o755);
  }
  return {
    directory,
    env: { PATH: `${bin}:/usr/bin:/bin` },
    calls: () => (existsSync(calls) ? readFileSync(calls, 'utf8').split('\n').filter((line) => line !== '').map((line) => line.split('\t')) : []),
  };
}

/** The result of a script. */
interface Run {
  status: number | null;
  stdout: string;
  stderr: string;
  calls: string[][];
}

/** Runs scripts/servers.sh with `args` on `machine`; `made` are the stand-ins when the test reads their files. */
function run(t: TestContext, machine: Machine, args: string[], made: Stand = stand(t, machine)): Run {
  const result = spawnSync('/bin/sh', [servers, ...args], { encoding: 'utf8', env: made.env });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, calls: made.calls() };
}

const present: Machine = { bay: true };

test('each verb of a project is one command of bay with the Compose file of the project', (t) => {
  const plan = run(t, present, ['plan', FILE]);
  assert.deepEqual(plan.calls, [['bay', '-f', FILE, 'plan', '--json']]);
  assert.deepEqual([plan.status, plan.stdout], [0, '{"command":"plan"}\n']);
  // No start pulls an image: up fails before it creates anything when the image is not installed.
  const up = run(t, present, ['up', FILE]);
  assert.deepEqual(up.calls, [['bay', '-f', FILE, 'up', '-no-pull', '--json']]);
  assert.deepEqual([up.status, up.stdout], [0, '{"command":"up"}\n']);
  assert.deepEqual(run(t, present, ['images', FILE]).calls, [['bay', '-f', FILE, 'images']]);
  assert.deepEqual(run(t, present, ['logs', FILE, 'node-0']).calls, [['bay', '-f', FILE, 'logs', '-f', 'node-0']]);
  assert.deepEqual(run(t, present, ['down', FILE]).calls, [['bay', '-f', FILE, 'down']]);
});

test('a command of bay that fails fails its verb with the status and the words of bay', (t) => {
  for (const verb of ['plan', 'up', 'images', 'down']) {
    const failed = run(t, { bay: true, status: { [verb]: 1 } }, [verb, FILE]);
    assert.deepEqual([failed.status, failed.stderr], [1, `bay: ${verb} fails\n`], verb);
  }
});

test('exec runs a command in the container of a service, with an empty standard input, and ends with its status', (t) => {
  const made = stand(t, present);
  const ran = run(t, present, ['exec', FILE, 'node-0', 'redis-cli', '--cluster', 'create', '192.0.2.1:6379', '--cluster-yes'], made);
  assert.deepEqual(ran.calls, [['bay', '-f', FILE, 'exec', 'node-0', 'redis-cli', '--cluster', 'create', '192.0.2.1:6379', '--cluster-yes']]);
  assert.equal(ran.status, 0);
  assert.equal(readFileSync(join(made.directory, 'input'), 'utf8'), '');
  assert.equal(run(t, { bay: true, status: { exec: 7 } }, ['exec', FILE, 'node-0', 'false']).status, 7);
});

test('pull fetches the image through bay, as the image of a project that the script writes and removes', (t) => {
  const made = stand(t, present);
  const pulled = run(t, present, ['pull', IMAGE], made);
  assert.equal(pulled.status, 0);
  assert.equal(pulled.calls.length, 1);
  const [program, option, file, command, ...rest] = pulled.calls[0] ?? [];
  assert.deepEqual([program, option, command, rest], ['bay', '-f', 'pull', []]);
  assert.match(file ?? '', /\/compose\.yaml$/);
  assert.equal(readFileSync(join(made.directory, 'pulled'), 'utf8'), `services:\n  redis:\n    image: "${IMAGE}"\n`);
  assert.equal(existsSync(dirname(file ?? '')), false);
  assert.equal(run(t, { bay: true, status: { pull: 1 } }, ['pull', IMAGE]).status, 1);
});

test('check passes when bay and the machine setup that a run needs are in place, and changes nothing', (t) => {
  const checked = run(t, present, ['check']);
  assert.deepEqual([checked.status, checked.stdout, checked.stderr], [0, '', '']);
  assert.deepEqual(checked.calls, [['bay', 'check', '-scope', 'run']]);
});

test('a machine setup of bay that is not in place fails check and names bay install', (t) => {
  const missing = run(t, { bay: true, status: { check: 1 } }, ['check']);
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /^scripts\/servers\.sh: the machine setup of bay is not in place: `bay install` applies it\n/);
  assert.match(missing.stderr, /bay: check fails\n$/);
  assert.deepEqual(missing.calls, [['bay', 'check', '-scope', 'run']]);
});

test('bay that PATH does not hold fails every verb, with its name and what installs it', (t) => {
  for (const args of [['check'], ['pull', IMAGE], ['plan', FILE], ['up', FILE], ['images', FILE], ['exec', FILE, 'a', 'true'], ['logs', FILE, 'a'], ['down', FILE]]) {
    const missing = run(t, { bay: false }, args);
    assert.equal(missing.status, 1, args.join(' '));
    assert.match(missing.stderr, /^scripts\/servers\.sh: bay is not found on PATH: install bay, /);
    assert.deepEqual([missing.stdout, missing.calls], ['', []]);
  }
});

test('a verb that the script does not know and a verb with other arguments fail', (t) => {
  for (const args of [[], ['start'], ['status'], ['present', IMAGE], ['check', 'a'], ['pull'], ['plan'], ['up'], ['up', 'a', 'b'], ['images'], ['exec', 'a', 'b'], ['logs', 'a'], ['down']]) {
    const unknown = run(t, present, args);
    assert.equal(unknown.status, 2, args.join(' '));
    assert.match(unknown.stderr, /^usage: scripts\/servers\.sh /);
    assert.deepEqual(unknown.calls, []);
  }
});

/**
 * Runs scripts/test-run.sh on `machine` with a command of sh. The command reads PROJECTS_SEEN, the file into which it
 * writes the directory of the projects of its run.
 */
function runTests(t: TestContext, machine: Machine, command: string): Run & { projects: string } {
  const made = stand(t, machine);
  const seen = join(made.directory, 'seen');
  const result = spawnSync('/bin/sh', [testRun, 'sh', '-c', command], { encoding: 'utf8', env: { ...made.env, PROJECTS_SEEN: seen } });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, calls: made.calls(), projects: existsSync(seen) ? readFileSync(seen, 'utf8').trim() : '' };
}

/** A command that leaves the Compose files of the projects `names` in the directory of its run. */
const leave = (names: string[]): string =>
  ['printf "%s\\n" "$SESSION_STORE_PROJECTS" > "$PROJECTS_SEEN"', ...names.map((name) => `mkdir "$SESSION_STORE_PROJECTS/${name}" && : > "$SESSION_STORE_PROJECTS/${name}/compose.yaml"`)].join(' && ');

test('a run checks the programs, gives the tests a directory of projects named after the run and removes it', (t) => {
  const ended = runTests(t, present, `${leave([])} && test -d "$SESSION_STORE_PROJECTS" && test -d "$TMPDIR"`);
  assert.deepEqual([ended.status, ended.stderr], [0, '']);
  assert.deepEqual(ended.calls, [['bay', 'check', '-scope', 'run']]);
  assert.match(ended.projects, /\/session-store-test-[0-9]+$/);
  assert.equal(existsSync(dirname(ended.projects)), false);
});

test('a run that ends or fails runs down for every Compose file that its directory still holds', (t) => {
  for (const status of [0, 3]) {
    const ended = runTests(t, present, `${leave(['p-1-0', 'p-1-1'])} && exit ${status}`);
    assert.deepEqual([ended.status, ended.stderr], [status, '']);
    assert.deepEqual(ended.calls, [
      ['bay', 'check', '-scope', 'run'],
      ['bay', '-f', join(ended.projects, 'p-1-0', 'compose.yaml'), 'down'],
      ['bay', '-f', join(ended.projects, 'p-1-1', 'compose.yaml'), 'down'],
    ]);
    assert.equal(existsSync(dirname(ended.projects)), false);
  }
});

test('a run whose programs or machine setup are missing fails before its command starts', (t) => {
  const withoutBay = runTests(t, { bay: false }, leave([]));
  assert.equal(withoutBay.status, 1);
  assert.match(withoutBay.stderr, /^scripts\/servers\.sh: bay is not found on PATH: /);
  assert.equal(withoutBay.projects, '');
  const withoutSetup = runTests(t, { bay: true, status: { check: 1 } }, leave([]));
  assert.equal(withoutSetup.status, 1);
  assert.match(withoutSetup.stderr, /`bay install` applies it/);
  assert.equal(withoutSetup.projects, '');
});

test('a project that down does not remove fails the run, which keeps its directory and names the file', (t) => {
  const kept = runTests(t, { bay: true, status: { down: 1 } }, leave(['p-1-0']));
  t.after(() => { if (kept.projects !== '') rmSync(dirname(kept.projects), { recursive: true, force: true }); });
  const file = join(kept.projects, 'p-1-0', 'compose.yaml');
  assert.equal(kept.status, 1);
  assert.equal(kept.stderr, `scripts/test-run.sh: the servers of ${file} are not removed; the directory of the run is kept:\nbay: down fails\n`);
  assert.equal(existsSync(file), true);
});

/**
 * A run that `signal` stops: its command leaves a project, reports that it started and then waits for a writer of a
 * FIFO, which never comes, so that only the signal ends it. The test sends the signal when it reads the report.
 */
function stopped(t: TestContext, signal: 'SIGINT' | 'SIGTERM'): Promise<Run & { projects: string }> {
  const made = stand(t, present);
  const seen = join(made.directory, 'seen');
  const fifo = join(made.directory, 'fifo');
  assert.equal(spawnSync('/usr/bin/mkfifo', [fifo]).status, 0);
  const child = spawn('/bin/sh', [testRun, 'sh', '-c', `${leave(['p-1-0'])} && echo started && read line < "$FIFO"`], {
    env: { ...made.env, PROJECTS_SEEN: seen, FIFO: fifo },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  let sent = false;
  child.stderr.on('data', (chunk: Buffer) => { stderr += chunk.toString(); });
  child.stdout.on('data', (chunk: Buffer) => {
    stdout += chunk.toString();
    if (!sent && stdout.includes('started\n')) {
      sent = true;
      child.kill(signal);
    }
  });
  return new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (status) => {
      resolve({ status, stdout, stderr, calls: made.calls(), projects: readFileSync(seen, 'utf8').trim() });
    });
  });
}

test('a run that SIGINT or SIGTERM stops waits for its command, runs down for its Compose files and removes its directory', async (t) => {
  for (const [signal, status] of [['SIGINT', 130], ['SIGTERM', 143]] as const) {
    const run = await stopped(t, signal);
    // The shell of the script reports the stopped job on standard error, in words of its own.
    assert.equal(run.status, status, signal);
    assert.deepEqual(run.calls, [['bay', 'check', '-scope', 'run'], ['bay', '-f', join(run.projects, 'p-1-0', 'compose.yaml'), 'down']]);
    assert.equal(existsSync(dirname(run.projects)), false);
  }
});
