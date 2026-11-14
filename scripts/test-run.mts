// Runs the tests of `make test` as a run that owns its servers and its temporary directory (SE-10.4).
//
//   node scripts/test-run.mts <command> [<argument> ...]
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
// scripts/servers.sh holds the commands. When bay or a part of the machine setup that a project needs is missing, the
// run fails before the command starts, with what is missing and what gives it.
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { constants, tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const NAME = 'scripts/test-run.mts';
const [program, ...args] = process.argv.slice(2);
if (program === undefined) {
  process.stderr.write(`usage: ${NAME} <command> [<argument> ...]\n`);
  process.exit(2);
}
const servers = join(dirname(fileURLToPath(import.meta.url)), 'servers.sh');

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

if (spawnSync(servers, ['check'], { stdio: ['ignore', 'inherit', 'inherit'] }).status !== 0) process.exit(1);
directory = mkdtempSync(join(tmpdir(), 'session-store-run-'));
projects = join(directory, `session-store-test-${process.pid}`);
mkdirSync(projects);
command = spawn(program, args, {
  detached: true,
  stdio: 'inherit',
  env: { ...process.env, SESSION_STORE_PROJECTS: projects, TMPDIR: directory },
});
command.on('error', (error) => {
  process.stderr.write(`${NAME}: ${program} does not start: ${error.message}\n`);
  end(127);
});
command.on('exit', (code, signal) => {
  end(stopping ?? code ?? 128 + constants.signals[signal ?? 'SIGKILL']);
});
