// Runs the tests of `make test` as a run that owns its servers and its temporary directory (SE-10.4).
//
//   node scripts/test-run.mts [--service redis] <command> [<argument> ...]
//
// The command runs with TMPDIR, a directory of this run, and SESSION_STORE_PROJECTS, the directory below it in which
// each test writes the Compose file of its servers, <project>/compose.yaml, before it starts them. The directory is
// named after the process of the run, session-store-test-<pid>, and a project is named after it, so no two runs of a
// machine hold one name.
//
// The command is the leader of a process group of its own, in a session of its own, so that a signal reaches every
// process of the command: a POSIX shell gives a script no way to make one, because set -m needs a terminal in dash,
// the /bin/sh of Debian and Ubuntu, and macOS has no setsid program. The run passes SIGINT and SIGTERM that it
// receives to that group, and the exit of the command is the event that it waits for. When the command ends or fails,
// and when the run is stopped, the run runs down for every Compose file that the directory still holds and removes the
// directory of the run. A project that down does not remove fails the run, which then keeps the directory and names
// the file.
//
// With --service redis the run starts the Redis service of the generated binding tests (SE-10.7) before the command:
// one Compose project in the directory of the projects, <directory of the projects>-redis, with the one service redis,
// redis-server of the image REDIS_IMAGE with the arguments of the server of the cases (SE-10.4). It reads the address
// of the service from the output of up and gives the command SESSION_STORE_REDIS_URL, redis://<address>, and
// SESSION_STORE_REDIS_STATE, the absolute path of scripts/redis-state.sh, the program of the state of the service. The
// project is removed with the other projects of the run.
//
// scripts/servers.sh holds the commands. When bay or a part of the machine setup that a project needs is missing, the
// run fails before the command starts, with what is missing and what gives it.
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { constants, tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const NAME = 'scripts/test-run.mts';
const given = process.argv.slice(2);
const service = given[0] === '--service' ? given[1] : undefined;
const [program, ...args] = service === undefined ? given : given.slice(2);
if (program === undefined || (given[0] === '--service' && service !== 'redis')) {
  process.stderr.write(`usage: ${NAME} [--service redis] <command> [<argument> ...]\n`);
  process.exit(2);
}
const scripts = dirname(fileURLToPath(import.meta.url));
const servers = join(scripts, 'servers.sh');
/** The version of the contract of bay whose output of up the run reads (SE-10.4). */
const SCHEMA = 1;
/** The arguments of a Redis server that keeps nothing on disk and takes connections from the host (SE-10.4). */
const PLAIN = ['--save', '', '--appendonly', 'no', '--protected-mode', 'no'];

/** The status of a run that a signal stopped: 128 and the number of the signal, as a shell gives it. */
const STOPPED = { SIGINT: 128 + constants.signals.SIGINT, SIGTERM: 128 + constants.signals.SIGTERM } as const;

let command: ChildProcess | undefined;
let stopping: number | undefined;
let directory: string | undefined;
let projects = '';

/** Removes the servers and the directory of the run; false when a project is not removed. */
function remove(): boolean {
  if (directory === undefined) return true;
  let kept = false;
  for (const name of existsSync(projects) ? readdirSync(projects).sort() : []) {
    const file = join(projects, name, 'compose.yaml');
    if (!existsSync(file)) continue;
    const down = spawnSync(servers, ['down', file], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    if (down.status !== 0) {
      const report = `${down.stdout ?? ''}${down.stderr ?? ''}${down.error === undefined ? '' : String(down.error)}`.replace(/\n+$/, '');
      process.stderr.write(`${NAME}: the servers of ${file} are not removed; the directory of the run is kept:\n${report}\n`);
      kept = true;
    }
  }
  if (kept) return false;
  rmSync(directory, { recursive: true, force: true });
  return true;
}

/** Ends the run with `status`, which a project that is not removed raises from 0 to 1. */
function end(status: number): never {
  const removed = remove();
  process.exit(!removed && status === 0 ? 1 : status);
}

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => {
    stopping ??= STOPPED[signal];
    // Before the command starts there is nothing to stop; the run ends at once.
    if (command?.pid === undefined) end(stopping);
    try {
      process.kill(-command.pid, signal);
    } catch (error) {
      // The group is gone when every process of the command has ended; its exit follows.
      if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error;
    }
  });
}

/** A scalar of the Compose file in double quotes; a control character and `$`, which Compose reads, are refused. */
function quote(text: string): string {
  if (/[\u0000-\u001f\u007f$]/.test(text)) {
    process.stderr.write(`${NAME}: ${JSON.stringify(text)} is not written into a Compose file: it holds a control character or $\n`);
    end(1);
  }
  return `"${text.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

/** The output of a command of scripts/servers.sh, which fails the run with the output of the command. */
function serversOutput(verb: string, file: string): string {
  const done = spawnSync(servers, [verb, file], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  if (done.status === 0) return done.stdout;
  const report = `${done.stdout ?? ''}${done.stderr ?? ''}${done.error === undefined ? '' : String(done.error)}`.replace(/\n+$/, '');
  throw new Error(`scripts/servers.sh ${verb} ${file} fails with ${String(done.status)}: ${report}`);
}

/** Starts the Redis service of the generated binding tests (SE-10.7) and gives its variables. */
function startRedis(): Record<string, string> {
  const image = process.env['REDIS_IMAGE'] ?? '';
  if (image === '') {
    process.stderr.write(`${NAME}: REDIS_IMAGE is not set: the Makefile names the pinned image of the Redis server\n`);
    end(1);
  }
  const project = join(projects, `${basename(projects)}-redis`);
  mkdirSync(project);
  const file = join(project, 'compose.yaml');
  writeFileSync(file, [
    'services:',
    '  "redis":',
    `    image: ${quote(image)}`,
    // The entry point of the image would run the server as the user redis; the server runs as root of its container.
    '    entrypoint: ["redis-server"]',
    `    command: [${PLAIN.map(quote).join(', ')}]`,
    '    expose: ["6379"]',
    '',
  ].join('\n'));
  let output: string;
  try {
    output = serversOutput('up', file);
  } catch (error) {
    // A start fails before it creates anything when the image is not installed.
    let installed = true;
    try {
      serversOutput('images', file);
    } catch {
      installed = false;
    }
    process.stderr.write(`${NAME}: the Redis service does not start${installed ? '' : `: the image ${image} is not installed, and make redis-image pulls it`}: ${(error as Error).message}\n`);
    end(1);
  }
  let document: { schemaVersion?: unknown; services?: { name?: unknown; address?: unknown }[] };
  try {
    document = JSON.parse(output) as typeof document;
  } catch {
    process.stderr.write(`${NAME}: \`bay up --json\` gives no JSON: ${output}\n`);
    end(1);
  }
  if (document.schemaVersion !== SCHEMA) {
    process.stderr.write(`${NAME}: \`bay up --json\` gives no document of schemaVersion ${SCHEMA}: ${output}\n`);
    end(1);
  }
  const address = document.services?.find((found) => found.name === 'redis')?.address;
  if (typeof address !== 'string' || address === '') {
    process.stderr.write(`${NAME}: \`bay up --json\` gives no address of the service redis: ${output}\n`);
    end(1);
  }
  return { SESSION_STORE_REDIS_URL: `redis://${address}`, SESSION_STORE_REDIS_STATE: join(scripts, 'redis-state.sh') };
}

if (spawnSync(servers, ['check'], { stdio: ['ignore', 'inherit', 'inherit'] }).status !== 0) process.exit(1);
directory = mkdtempSync(join(tmpdir(), 'session-store-run-'));
projects = join(directory, `session-store-test-${process.pid}`);
mkdirSync(projects);
const variables = service === 'redis' ? startRedis() : {};
command = spawn(program, args, {
  detached: true,
  stdio: 'inherit',
  env: { ...process.env, ...variables, SESSION_STORE_PROJECTS: projects, TMPDIR: directory },
});
command.on('error', (error) => {
  process.stderr.write(`${NAME}: ${program} does not start: ${error.message}\n`);
  end(127);
});
command.on('exit', (code, signal) => {
  end(stopping ?? code ?? 128 + constants.signals[signal ?? 'SIGKILL']);
});
