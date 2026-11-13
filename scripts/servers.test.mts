// The commands of scripts/servers.sh and the run of scripts/test-run.sh (SE-10.4): bay starts and removes the servers
// of the tests, and the engine of the platform, Apple container on Darwin and docker on Linux, pulls and finds the
// image. The test runs the scripts with stand-ins that it writes into a directory of its own, the only programs of
// PATH besides /usr/bin and /bin: a uname that names the platform, and a bay and an engine that record their
// arguments, so it needs neither bay nor a container runtime.
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

/** What the stand-ins of one test do. */
interface Machine {
  /** The platform that the stand-in uname names. */
  system: string;
  /** The engine that PATH holds, none when it is undefined. */
  engine?: string;
  /** Whether PATH holds bay. */
  bay: boolean;
  /** The exit status of the stand-in bay for each command that is named; 0 for the others. */
  status?: Record<string, number>;
}

/** The directory of the stand-ins, with the commands that they received. */
interface Stand {
  directory: string;
  env: Record<string, string>;
  /** The commands that bay and the engine received so far, each as its words. */
  calls: () => string[][];
}

/** Writes the stand-ins of `machine` into a directory that the test removes. */
function stand(t: TestContext, machine: Machine): Stand {
  const directory = mkdtempSync(join(tmpdir(), 'session-store-servers-'));
  t.after(() => { rmSync(directory, { recursive: true, force: true }); });
  const bin = join(directory, 'bin');
  mkdirSync(bin);
  const write = (name: string, text: string): void => {
    writeFileSync(join(bin, name), text);
    chmodSync(join(bin, name), 0o755);
  };
  write('uname', `#!/bin/sh\necho ${machine.system}\n`);
  const calls = join(directory, 'calls');
  // Each command is one line of the file calls, its words joined by a tab.
  const record = `line=$(basename "$0")\nfor word in "$@"; do line="$line\t$word"; done\nprintf '%s\\n' "$line" >> '${calls}'\n`;
  if (machine.engine !== undefined) write(machine.engine, `#!/bin/sh\n${record}`);
  if (machine.bay) {
    // The command of bay is its first word that is not the option -f or the value of that option.
    const cases = Object.entries(machine.status ?? {}).map(([command, status]) => `${command}) echo "bay: ${command} fails" >&2; exit ${status} ;;`).join('\n');
    write('bay', `#!/bin/sh\n${record}if [ "$1" = -f ]; then shift 2; fi\ncase "$1" in\n${cases}\nstatus) echo '{"groups":[]}' ;;\ncheck) echo 'ok       dns agent' ;;\nesac\nexit 0\n`);
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

/** Runs scripts/servers.sh with `args` on `machine`. */
function run(t: TestContext, machine: Machine, args: string[]): Run {
  const made = stand(t, machine);
  const result = spawnSync('/bin/sh', [servers, ...args], { encoding: 'utf8', env: made.env });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, calls: made.calls() };
}

const darwin: Machine = { system: 'Darwin', engine: 'container', bay: true };
const linux: Machine = { system: 'Linux', engine: 'docker', bay: true };

test('the servers start, stop and report through bay, with the Compose file of the project', (t) => {
  for (const machine of [darwin, linux]) {
    assert.deepEqual(run(t, machine, ['up', '/run/p/compose.yaml']).calls, [['bay', '-f', '/run/p/compose.yaml', 'up']]);
    assert.deepEqual(run(t, machine, ['down', '/run/p/compose.yaml']).calls, [['bay', '-f', '/run/p/compose.yaml', 'down']]);
    assert.deepEqual(run(t, machine, ['logs', '/run/p/compose.yaml', 'node-0']).calls, [['bay', '-f', '/run/p/compose.yaml', 'logs', '-f', 'node-0']]);
    const status = run(t, machine, ['status']);
    assert.deepEqual(status.calls, [['bay', 'status', '--json']]);
    assert.deepEqual([status.status, status.stdout], [0, '{"groups":[]}\n']);
  }
});

test('a command of bay that fails fails its verb with the status and the words of bay', (t) => {
  const failed = run(t, { ...darwin, status: { up: 1 } }, ['up', '/run/p/compose.yaml']);
  assert.deepEqual([failed.status, failed.stderr], [1, 'bay: up fails\n']);
});

test('the engine of the platform pulls the image and finds it', (t) => {
  assert.deepEqual(run(t, darwin, ['pull', IMAGE]).calls, [['container', 'image', 'pull', IMAGE]]);
  assert.deepEqual(run(t, darwin, ['present', IMAGE]).calls, [['container', 'image', 'inspect', IMAGE]]);
  assert.deepEqual(run(t, linux, ['pull', IMAGE]).calls, [['docker', 'image', 'pull', IMAGE]]);
  assert.deepEqual(run(t, linux, ['present', IMAGE]).calls, [['docker', 'image', 'inspect', IMAGE]]);
});

test('check passes when bay, the engine and the machine setup of bay are in place, and changes nothing', (t) => {
  for (const machine of [darwin, linux]) {
    const checked = run(t, machine, ['check']);
    assert.deepEqual([checked.status, checked.stdout, checked.stderr], [0, '', '']);
    assert.deepEqual(checked.calls, [['bay', 'check']]);
  }
});

test('a machine setup of bay that is not in place fails check and names bay install', (t) => {
  const missing = run(t, { ...darwin, status: { check: 1 } }, ['check']);
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /^scripts\/servers\.sh: the machine setup of bay is not in place: `bay install` applies it\n/);
  assert.match(missing.stderr, /bay: check fails\n$/);
  assert.deepEqual(missing.calls, [['bay', 'check']]);
});

test('a program that PATH does not hold fails the verbs that need it, with its name and what installs it', (t) => {
  for (const args of [['check'], ['up', '/run/p/compose.yaml'], ['down', '/run/p/compose.yaml'], ['status'], ['logs', '/run/p/compose.yaml', 'a']]) {
    const missing = run(t, { ...darwin, bay: false }, args);
    assert.equal(missing.status, 1, args.join(' '));
    assert.match(missing.stderr, /^scripts\/servers\.sh: bay is not found on PATH: install bay, /);
    assert.deepEqual([missing.stdout, missing.calls], ['', []]);
  }
  for (const [system, engine, install] of [['Linux', 'docker', 'Docker Engine'], ['Darwin', 'container', 'Apple container']] as const) {
    for (const args of [['check'], ['pull', IMAGE], ['present', IMAGE]]) {
      const missing = run(t, { system, bay: true }, args);
      assert.equal(missing.status, 1, `${system} ${args.join(' ')}`);
      assert.match(missing.stderr, new RegExp(`^scripts/servers\\.sh: ${engine} is not found on PATH: install ${install} `));
      assert.deepEqual([missing.stdout, missing.calls], ['', []]);
    }
  }
});

test('a platform without an engine and a verb that the script does not know fail', (t) => {
  const other = run(t, { system: 'FreeBSD', engine: 'docker', bay: true }, ['check']);
  assert.equal(other.status, 1);
  assert.match(other.stderr, /^scripts\/servers\.sh: no engine is named for FreeBSD: Darwin runs Apple container and Linux runs docker\n$/);
  assert.deepEqual(other.calls, []);
  for (const args of [[], ['start'], ['check', 'a'], ['pull'], ['present'], ['up'], ['up', 'a', 'b'], ['down'], ['status', 'a'], ['logs', 'a']]) {
    const unknown = run(t, linux, args);
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
  const ended = runTests(t, darwin, `${leave([])} && test -d "$SESSION_STORE_PROJECTS" && test -d "$TMPDIR"`);
  assert.deepEqual([ended.status, ended.stderr], [0, '']);
  assert.deepEqual(ended.calls, [['bay', 'check']]);
  assert.match(ended.projects, /\/session-store-test-[0-9]+$/);
  assert.equal(existsSync(dirname(ended.projects)), false);
});

test('a run that ends or fails runs down for every Compose file that its directory still holds', (t) => {
  for (const status of [0, 3]) {
    const ended = runTests(t, linux, `${leave(['p-1-0', 'p-1-1'])} && exit ${status}`);
    assert.deepEqual([ended.status, ended.stderr], [status, '']);
    assert.deepEqual(ended.calls, [
      ['bay', 'check'],
      ['bay', '-f', join(ended.projects, 'p-1-0', 'compose.yaml'), 'down'],
      ['bay', '-f', join(ended.projects, 'p-1-1', 'compose.yaml'), 'down'],
    ]);
    assert.equal(existsSync(dirname(ended.projects)), false);
  }
});

test('a run whose programs or machine setup are missing fails before its command starts', (t) => {
  const withoutBay = runTests(t, { ...darwin, bay: false }, leave([]));
  assert.equal(withoutBay.status, 1);
  assert.match(withoutBay.stderr, /^scripts\/servers\.sh: bay is not found on PATH: /);
  assert.equal(withoutBay.projects, '');
  const withoutSetup = runTests(t, { ...darwin, status: { check: 1 } }, leave([]));
  assert.equal(withoutSetup.status, 1);
  assert.match(withoutSetup.stderr, /`bay install` applies it/);
  assert.equal(withoutSetup.projects, '');
});

test('a project that down does not remove fails the run, which keeps its directory and names the file', (t) => {
  const kept = runTests(t, { ...darwin, status: { down: 1 } }, leave(['p-1-0']));
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
  const made = stand(t, darwin);
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
    assert.deepEqual([run.status, run.stderr], [status, ''], signal);
    assert.deepEqual(run.calls, [['bay', 'check'], ['bay', '-f', join(run.projects, 'p-1-0', 'compose.yaml'), 'down']]);
    assert.equal(existsSync(dirname(run.projects)), false);
  }
});
