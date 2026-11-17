// The commands of scripts/servers.sh and the run of scripts/test-run.mts (SE-10.4): bay starts and removes the servers
// of the tests, gives their names and addresses, and pulls and finds the image. The test runs the scripts with a
// stand-in of bay that it writes into a directory of its own, the only program of PATH besides /usr/bin and /bin, which
// records its arguments, so it needs neither bay nor a container runtime.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';

const scripts = dirname(fileURLToPath(import.meta.url));
const servers = join(scripts, 'servers.sh');
const testRun = join(scripts, 'test-run.mts');
const IMAGE = 'docker.io/library/redis:1.2.3@sha256:00';
const FILE = '/run/p/compose.yaml';

/** What the stand-in of one test does. */
interface Machine {
  /** Whether PATH holds bay. */
  bay: boolean;
  /** The exit status of the stand-in bay for each command that is named; 0 for the others. */
  status?: Record<string, number>;
  /** The document that up prints, in place of one that names the command. */
  up?: string;
}

/** The line that the stand-in bay gives of its version. */
const VERSION = 'bay v0.0.0-20260101000000-000000000000+dirty (000000000000, modified) go1.0.0 linux/arm64 schema 1';
/** The two commands of a check that passes. */
const CHECKED = [['bay', 'version'], ['bay', 'check', '-scope', 'run']];

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
    // after the option -f and its value. pull keeps the Compose file that it received, exec keeps its standard input, or
    // the word pipe when that input is a pipe,
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
      `exec) if [ -p /dev/stdin ]; then echo pipe; else cat; fi > '${join(directory, 'input')}' ;;`,
      ...(machine.up === undefined ? [] : [`up) cat "$file" > '${join(directory, 'started')}'; printf '%s\\n' '${machine.up}' ;;`]),
      `plan | up) echo "{\\"command\\":\\"$1\\"}" ;;`,
      `check) echo 'ok       dns agent' ;;`,
      `version) echo '${VERSION}' ;;`,
      'esac',
      'exit 0',
      '',
    ].join('\n'));
    chmodSync(join(bin, 'bay'), 0o755);
  }
  // A program that starts another one, as sudo does: it records its words and runs the words after its option.
  writeFileSync(join(bin, 'wrap'), `#!/bin/sh\nline=wrap\nfor word in "$@"; do line="$line\t$word"; done\nprintf '%s\\n' "$line" >> '${calls}'\nshift\nexec "$@"\n`);
  chmodSync(join(bin, 'wrap'), 0o755);
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
function run(t: TestContext, machine: Machine, args: string[], made: Stand = stand(t, machine), env: Record<string, string> = {}): Run {
  const result = spawnSync('/bin/sh', [servers, ...args], { encoding: 'utf8', env: { ...made.env, ...env } });
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
  assert.deepEqual(run(t, present, ['output', FILE, 'node-0']).calls, [['bay', '-f', FILE, 'logs', 'node-0']]);
  assert.deepEqual(run(t, present, ['down', FILE]).calls, [['bay', '-f', FILE, 'down']]);
});

test('a command of bay that fails fails its verb with the status and the words of bay', (t) => {
  for (const verb of ['plan', 'up', 'images', 'down']) {
    const failed = run(t, { bay: true, status: { [verb]: 1 } }, [verb, FILE]);
    assert.deepEqual([failed.status, failed.stderr], [1, `bay: ${verb} fails\n`], verb);
  }
});

test('exec runs a command in the container of a service, with the empty standard input of every verb, and ends with its status', (t) => {
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
  assert.equal(pulled.calls.length, 2);
  const [program, option, file, command, ...rest] = pulled.calls[0] ?? [];
  // After the pull, the store is asked for the image of the same project.
  assert.deepEqual(pulled.calls[1], ['bay', '-f', file, 'images']);
  assert.deepEqual([program, option, command, rest], ['bay', '-f', 'pull', []]);
  assert.match(file ?? '', /\/compose\.yaml$/);
  // bay names the project after the directory and refuses a name with another character than these.
  assert.match(basename(dirname(file ?? '')), /^session-store-pull-[A-Za-z0-9]+$/);
  assert.equal(readFileSync(join(made.directory, 'pulled'), 'utf8'), `services:\n  redis:\n    image: "${IMAGE}"\n`);
  assert.equal(existsSync(dirname(file ?? '')), false);
  assert.equal(run(t, { bay: true, status: { pull: 1 } }, ['pull', IMAGE]).status, 1);
});

test('pull fails and names the image when bay ends the pull but the local store does not hold the image', (t) => {
  const failed = run(t, { bay: true, status: { images: 1 } }, ['pull', IMAGE]);
  assert.equal(failed.status, 1);
  assert.deepEqual(failed.calls.map((call) => call[3]), ['pull', 'images']);
  assert.match(failed.stderr, new RegExp(`scripts/servers\\.sh: bay pull ended, but the local store does not hold ${IMAGE.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\n$`));
});

test('check writes the version of bay, whatever build it is, and passes when the machine setup that a run needs is in place', (t) => {
  const checked = run(t, present, ['check']);
  assert.deepEqual([checked.status, checked.stdout, checked.stderr], [0, `${VERSION}\n`, '']);
  assert.deepEqual(checked.calls, CHECKED);
});

test('a bay that gives no version fails check before the setup is asked', (t) => {
  const failed = run(t, { bay: true, status: { version: 2 } }, ['check']);
  assert.equal(failed.status, 1);
  assert.match(failed.stderr, /^bay: version fails\nscripts\/servers\.sh: bay gives no version: install a release of bay that has the command version\n$/);
  assert.deepEqual(failed.calls, [['bay', 'version']]);
});

test('a machine setup of bay that is not in place fails check and names bay install', (t) => {
  const missing = run(t, { bay: true, status: { check: 1 } }, ['check']);
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /^scripts\/servers\.sh: the machine setup of bay is not in place: `bay install` applies it\n/);
  assert.match(missing.stderr, /bay: check fails\n$/);
  assert.deepEqual(missing.calls, CHECKED);
});

test('BAY names the words that start bay, and every verb starts bay by them', (t) => {
  const down = run(t, present, ['down', FILE], undefined, { BAY: 'wrap -n bay' });
  assert.deepEqual([down.status, down.calls], [0, [['wrap', '-n', 'bay', '-f', FILE, 'down'], ['bay', '-f', FILE, 'down']]]);
  const checked = run(t, present, ['check'], undefined, { BAY: 'wrap -n bay' });
  assert.deepEqual([checked.status, checked.calls.filter(([program]) => program === 'bay')], [0, CHECKED]);
  const pulled = run(t, present, ['pull', IMAGE], undefined, { BAY: 'wrap -n bay' });
  assert.deepEqual([pulled.status, pulled.calls.map(([program]) => program)], [0, ['wrap', 'bay', 'wrap', 'bay']]);
  const missing = run(t, present, ['down', FILE], undefined, { BAY: 'absent -n bay' });
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /^scripts\/servers\.sh: absent is not found on PATH: /);
  assert.deepEqual(missing.calls, []);
});

test('bay that PATH does not hold fails every verb, with its name and what installs it', (t) => {
  for (const args of [['check'], ['pull', IMAGE], ['plan', FILE], ['up', FILE], ['images', FILE], ['exec', FILE, 'a', 'true'], ['logs', FILE, 'a'], ['output', FILE, 'a'], ['down', FILE]]) {
    const missing = run(t, { bay: false }, args);
    assert.equal(missing.status, 1, args.join(' '));
    assert.match(missing.stderr, /^scripts\/servers\.sh: bay is not found on PATH: install bay, /);
    assert.deepEqual([missing.stdout, missing.calls], ['', []]);
  }
});

test('a verb that the script does not know and a verb with other arguments fail', (t) => {
  for (const args of [[], ['start'], ['status'], ['present', IMAGE], ['check', 'a'], ['pull'], ['plan'], ['up'], ['up', 'a', 'b'], ['images'], ['exec', 'a', 'b'], ['logs', 'a'], ['output', 'a'], ['down']]) {
    const unknown = run(t, present, args);
    assert.equal(unknown.status, 2, args.join(' '));
    assert.match(unknown.stderr, /^usage: scripts\/servers\.sh /);
    assert.deepEqual(unknown.calls, []);
  }
});

/**
 * The shells of the commands of a run: /bin/sh of the machine and dash, which is /bin/sh of Debian and Ubuntu and
 * which has no job control without a terminal. The node of the test starts the run, as the node of the checkout does
 * in make test.
 */
const SHELLS = ['/bin/sh', '/bin/dash'].filter((shell) => existsSync(shell));

/**
 * Runs scripts/test-run.mts on `machine` with a command of `shell`. The command reads PROJECTS_SEEN, the file into which it
 * writes the directory of the projects of its run.
 */
function runTests(t: TestContext, machine: Machine, command: string, shell = '/bin/sh', options: string[] = [], env: Record<string, string> = {}, made = stand(t, machine)): Run & { projects: string } {
  const seen = join(made.directory, 'seen');
  const result = spawnSync(process.execPath, [testRun, ...options, shell, '-c', command], { encoding: 'utf8', env: { ...made.env, ...env, PROJECTS_SEEN: seen } });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, calls: made.calls(), projects: existsSync(seen) ? readFileSync(seen, 'utf8').trim() : '' };
}

/** A command that leaves the Compose files of the projects `names` in the directory of its run. */
const leave = (names: string[]): string =>
  ['printf "%s\\n" "$SESSION_STORE_PROJECTS" > "$PROJECTS_SEEN"', ...names.map((name) => `mkdir "$SESSION_STORE_PROJECTS/${name}" && : > "$SESSION_STORE_PROJECTS/${name}/compose.yaml"`)].join(' && ');

test('a run checks the programs, gives the tests a directory of projects named after the run and removes it', (t) => {
  for (const shell of SHELLS) {
    const ended = runTests(t, present, `${leave([])} && test -d "$SESSION_STORE_PROJECTS" && test -d "$TMPDIR"`, shell);
    // The first line of a run is the version of the bay that starts its servers.
    assert.deepEqual([ended.status, ended.stdout, ended.stderr], [0, `${VERSION}\n`, ''], shell);
    assert.deepEqual(ended.calls, CHECKED);
    assert.match(ended.projects, /\/session-store-test-[0-9]+$/);
    assert.equal(existsSync(dirname(ended.projects)), false);
  }
});

test('a run that ends or fails runs down for every Compose file that its directory still holds', (t) => {
  for (const [shell, status] of SHELLS.flatMap((shell) => [[shell, 0], [shell, 3]] as const)) {
    const ended = runTests(t, present, `${leave(['p-1-0', 'p-1-1'])} && exit ${status}`, shell);
    assert.deepEqual([ended.status, ended.stderr], [status, ''], shell);
    assert.deepEqual(ended.calls, [
      ...CHECKED,
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
  for (const shell of SHELLS) {
    const kept = runTests(t, { bay: true, status: { down: 1 } }, leave(['p-1-0']), shell);
    t.after(() => { if (kept.projects !== '') rmSync(dirname(kept.projects), { recursive: true, force: true }); });
    const file = join(kept.projects, 'p-1-0', 'compose.yaml');
    assert.equal(kept.status, 1, shell);
    assert.equal(kept.stderr, `scripts/test-run.mts: the servers of ${file} are not removed; the directory of the run is kept:\nbay: down fails\n`, shell);
    assert.equal(existsSync(file), true);
  }
});

/** The output of up of a project whose service redis bay started. */
const STARTED = JSON.stringify({ schemaVersion: 1, services: [{ name: 'redis', address: 'redis.p.test:6379', ipv4: '10.0.0.2' }] });

test('a run with the service redis starts its server before the command, gives its address and its program and removes it', (t) => {
  const made = stand(t, { bay: true, up: STARTED });
  const variables = 'printf "%s %s\\n" "$SESSION_STORE_REDIS_URL" "$SESSION_STORE_REDIS_STATE"';
  const ended = runTests(t, present, `${leave([])} && ${variables}`, '/bin/sh', ['--service', 'redis'], { REDIS_IMAGE: IMAGE }, made);
  const file = join(ended.projects, `${basename(ended.projects)}-redis`, 'compose.yaml');
  assert.deepEqual([ended.status, ended.stderr], [0, '']);
  assert.equal(ended.stdout, `${VERSION}\nredis://redis.p.test:6379 ${join(dirname(testRun), 'redis-state.sh')}\n`);
  assert.deepEqual(ended.calls, [...CHECKED, ['bay', '-f', file, 'up', '-no-pull', '--json'], ['bay', '-f', file, 'down']]);
  assert.equal(readFileSync(join(made.directory, 'started'), 'utf8'), [
    'services:',
    '  "redis":',
    `    image: "${IMAGE}"`,
    '    entrypoint: ["redis-server"]',
    '    command: ["--save", "", "--appendonly", "no", "--protected-mode", "no"]',
    '    expose: ["6379"]',
    '',
  ].join('\n'));
  assert.equal(existsSync(dirname(ended.projects)), false);
});

test('a service that does not start, or whose start gives another document, fails the run before its command and is removed', (t) => {
  const command = `${leave([])} && echo ran`;
  const withoutImage = runTests(t, present, command, '/bin/sh', ['--service', 'redis']);
  assert.deepEqual([withoutImage.status, withoutImage.stdout], [1, `${VERSION}\n`]);
  assert.match(withoutImage.stderr, /REDIS_IMAGE is not set/);
  const missing = runTests(t, { bay: true, status: { up: 1, images: 1 } }, command, '/bin/sh', ['--service', 'redis'], { REDIS_IMAGE: IMAGE });
  assert.deepEqual([missing.status, missing.stdout], [1, `${VERSION}\n`]);
  assert.match(missing.stderr, /the image .* is not installed, and make redis-image pulls it/);
  assert.deepEqual(missing.calls.map((words) => words.at(-1)), ['version', 'run', '--json', 'images', 'down']);
  for (const document of [JSON.stringify({ schemaVersion: 2, services: [] }), JSON.stringify({ schemaVersion: 1, services: [] })]) {
    const other = runTests(t, { bay: true, up: document }, command, '/bin/sh', ['--service', 'redis'], { REDIS_IMAGE: IMAGE });
    assert.deepEqual([other.status, other.stdout], [1, `${VERSION}\n`], document);
    assert.match(other.stderr, /`bay up --json` gives no (document of schemaVersion 1|address of the service redis)/, document);
    assert.deepEqual(other.calls.at(-1)?.at(-1), 'down');
  }
  const unknown = runTests(t, present, command, '/bin/sh', ['--service', 'postgres']);
  assert.deepEqual([unknown.status, unknown.calls], [2, []]);
});

/**
 * A run that `signal` stops: its command leaves a project, reports that it started and then waits for a writer of a
 * FIFO, which never comes, so that only the signal ends it. The test sends the signal when it reads the report.
 */
function stopped(t: TestContext, signal: 'SIGINT' | 'SIGTERM', shell: string): Promise<Run & { projects: string }> {
  const made = stand(t, present);
  const seen = join(made.directory, 'seen');
  const fifo = join(made.directory, 'fifo');
  assert.equal(spawnSync('/usr/bin/mkfifo', [fifo]).status, 0);
  const child = spawn(process.execPath, [testRun, shell, '-c', `${leave(['p-1-0'])} && echo started && read line < "$FIFO"`], {
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

test('the command of a run leads a process group of its own, so that a signal of the run reaches all its processes', (t) => {
  for (const shell of SHELLS) {
    const ended = runTests(t, present, `${leave([])} && test "$(ps -o pgid= -p $$ | tr -d ' ')" = "$$"`, shell);
    assert.deepEqual([ended.status, ended.stderr], [0, ''], shell);
  }
});

test('a run that SIGINT or SIGTERM stops waits for its command, runs down for its Compose files and removes its directory', async (t) => {
  for (const [shell, signal, status] of SHELLS.flatMap((shell) => [[shell, 'SIGINT', 130], [shell, 'SIGTERM', 143]] as const)) {
    const run = await stopped(t, signal, shell);
    // The shell of the command may report its interrupted read in words of its own; the run writes nothing.
    assert.equal(run.status, status, `${shell} ${signal}`);
    assert.doesNotMatch(run.stderr, /test-run|job control/, `${shell} ${signal}`);
    assert.deepEqual(run.calls, [...CHECKED, ['bay', '-f', join(run.projects, 'p-1-0', 'compose.yaml'), 'down']]);
    assert.equal(existsSync(dirname(run.projects)), false);
  }
});

test('the test of TLS trusts bay through the trust store of the machine and changes no environment', () => {
  // SE-10.4: the plan names the authority file below the home of the user who runs bay, which is root on the runner,
  // and the test process does not run as root. The test trusts the authority through the trust store of the machine
  // that `bay install` writes, so no test reads that file, names a file in SSL_CERT_FILE or SSL_CERT_DIR, or changes
  // the environment that the trials of its process read in parallel.
  const tests = join(scripts, '..', 'packages', 'redis', 'rust', 'tests');
  const files = [
    ...readdirSync(tests).filter((name) => name.endsWith('.rs')),
    ...['servers', 'store'].flatMap((directory) => readdirSync(join(tests, directory)).map((name) => join(directory, name))),
  ];
  assert.ok(files.includes('server.rs') && files.includes('contract.rs'), `the tests of the store of Redis are not in ${tests}`);
  const found: string[] = [];
  for (const name of files) {
    const text = readFileSync(join(tests, name), 'utf8');
    for (const pattern of [/\bset_var\(/, /\bremove_var\(/, /SSL_CERT_FILE|SSL_CERT_DIR/, /"ca"/]) {
      if (pattern.test(text)) found.push(`${name}: ${pattern.source}`);
    }
  }
  assert.deepEqual(found, [], 'a test names the authority file of bay or changes the environment of its process');
  // The trial of TLS is one of the trials of tests/server.rs.
  assert.match(readFileSync(join(tests, 'server.rs'), 'utf8'), /Trial::test\("a_tls_connection_reads_and_writes"/);
  assert.equal(files.includes('tls.rs'), false, 'tests/tls.rs is a test binary of its own');
});

/** The start of the process `pid` as ps gives it, the mark of a run beside its process id; empty when no process has it. */
const startOf = (pid: number | string): string => spawnSync('ps', ['-o', 'lstart=', '-p', String(pid)], { encoding: 'utf8' }).stdout.trim();

test('a run removes the servers and the directory of a run whose process is gone, and leaves a run that lives', (t) => {
  // T1.77: a run that SIGKILL ends, with its command, removes nothing, and no process can: its servers stay, 7 of them
  // after a kill of make test. The directory of that run stays too, with the mark of the run, the id and the start of
  // its process, and the Compose file of each project, so the next run runs down for each, removes the directory and
  // says so, as the run itself does at its end. A process id is used again, so a run is gone when no process has its
  // id or the process that has it started at another time. A run whose process lives is left as it is, and so is a
  // directory without a mark, which is no run that this program can judge.
  const made = stand(t, present);
  const temporary = join(made.directory, 'tmp');
  const gone = spawnSync('/bin/sh', ['-c', 'echo $$'], { encoding: 'utf8' }).stdout.trim();
  const runs = {
    dead: { pid: Number(gone), started: 'Thu Jan  1 00:00:00 1970', projects: ['p-2-0', 'p-2-1'] },
    reused: { pid: process.pid, started: 'Thu Jan  1 00:00:00 1970', projects: ['p-4-0'] },
    alive: { pid: process.pid, started: startOf(process.pid), projects: ['p-3-0'] },
    unmarked: { pid: Number(gone), started: undefined, projects: ['p-5-0'] },
  };
  const folder = (name: keyof typeof runs): string => join(temporary, `session-store-run-${name}`);
  const file = (name: keyof typeof runs, project: string): string => join(folder(name), `session-store-test-${runs[name].pid}`, project, 'compose.yaml');
  for (const name of Object.keys(runs) as (keyof typeof runs)[]) {
    for (const project of runs[name].projects) {
      mkdirSync(dirname(file(name, project)), { recursive: true });
      writeFileSync(file(name, project), 'services: {}\n');
    }
    if (runs[name].started !== undefined) writeFileSync(join(folder(name), 'run.json'), JSON.stringify({ pid: runs[name].pid, started: runs[name].started }));
  }
  assert.notEqual(runs.alive.started, '');
  const ended = runTests(t, present, `${leave([])}`, '/bin/sh', [], { TMPDIR: temporary }, made);
  assert.equal(ended.status, 0);
  assert.equal(ended.stderr, [
    `scripts/test-run.mts: the run of the process ${gone} is gone; its 2 projects and its directory ${folder('dead')} are removed`,
    `scripts/test-run.mts: the run of the process ${process.pid} is gone; its 1 project and its directory ${folder('reused')} are removed`,
    '',
  ].join('\n'));
  assert.deepEqual(ended.calls, [
    ...CHECKED,
    ['bay', '-f', file('dead', 'p-2-0'), 'down'],
    ['bay', '-f', file('dead', 'p-2-1'), 'down'],
    ['bay', '-f', file('reused', 'p-4-0'), 'down'],
  ]);
  assert.equal(existsSync(folder('dead')), false, 'the directory of the run that is gone stays');
  assert.equal(existsSync(folder('reused')), false, 'the directory of the run whose process id another process has stays');
  assert.equal(existsSync(file('alive', 'p-3-0')), true, 'the directory of a run that lives is removed');
  assert.equal(existsSync(file('unmarked', 'p-5-0')), true, 'a directory without the mark of a run is removed');
});
