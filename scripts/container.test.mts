// The commands of scripts/container.sh (SE-10.4): the container program is the one of the platform, Apple container on
// Darwin and docker on Linux, and each verb of the script is one command of that program. The test runs the script
// with stand-ins that it writes into a directory of its own, the only programs of PATH besides /usr/bin and /bin: a
// uname that names the platform and a container program that records its arguments, so it needs no container runtime.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = join(dirname(fileURLToPath(import.meta.url)), 'container.sh');
const IMAGE = 'docker.io/library/redis:1.2.3@sha256:00';

/** The result of the script with `arguments` on the platform `system`, where `program` is the container program found. */
interface Run {
  status: number | null;
  stdout: string;
  stderr: string;
  /** The commands that the container program received, each as its words. */
  calls: string[][];
}

/**
 * Runs the script on the platform that the stand-in uname names. `program` is the container program that PATH holds,
 * none when it is undefined; `names` are the lines that it writes for a command that lists containers.
 */
function run(t: TestContext, system: string, program: string | undefined, args: string[], names: string[] = []): Run {
  const directory = mkdtempSync(join(tmpdir(), 'session-store-container-'));
  t.after(() => { rmSync(directory, { recursive: true, force: true }); });
  const write = (name: string, text: string): void => {
    writeFileSync(join(directory, name), text);
    chmodSync(join(directory, name), 0o755);
  };
  write('uname', `#!/bin/sh\necho ${system}\n`);
  const calls = join(directory, 'calls');
  // Each command is one line of the file calls, its words joined by a tab; ls and ps write the names.
  if (program !== undefined) {
    writeFileSync(join(directory, 'names'), names.map((name) => `${name}\n`).join(''));
    write(program, `#!/bin/sh\nline=$(basename "$0")\nfor word in "$@"; do line="$line\t$word"; done\nprintf '%s\\n' "$line" >> '${calls}'\ncase "$1" in ls|ps) cat '${join(directory, 'names')}' ;; esac\n`);
  }
  const result = spawnSync('/bin/sh', [script, ...args], { encoding: 'utf8', env: { PATH: `${directory}:/usr/bin:/bin` } });
  const recorded = existsSync(calls) ? readFileSync(calls, 'utf8').split('\n').filter((line) => line !== '').map((line) => line.split('\t')) : [];
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, calls: recorded };
}

test('on Linux each verb is one command of docker', (t) => {
  const linux = (args: string[], names: string[] = []): Run => run(t, 'Linux', 'docker', args, names);
  assert.deepEqual([linux(['program']).stdout, linux(['program']).calls], ['docker\n', []]);
  assert.deepEqual(linux(['pull', IMAGE]).calls, [['docker', 'image', 'pull', IMAGE]]);
  assert.deepEqual(linux(['present', IMAGE]).calls, [['docker', 'image', 'inspect', IMAGE]]);
  assert.deepEqual(linux(['run', 'run-1-0', '--volume', '/tmp/a:/mnt/session-store', '--entrypoint', 'sh', IMAGE, '-c', 'hostname -i && exec redis-server "$@"', 'redis-server', '--save', '']).calls, [
    ['docker', 'run', '--rm', '--name', 'run-1-0', '--volume', '/tmp/a:/mnt/session-store', '--entrypoint', 'sh', IMAGE, '-c', 'hostname -i && exec redis-server "$@"', 'redis-server', '--save', ''],
  ]);
  assert.deepEqual(linux(['delete', 'run-1-0', 'run-1-1']).calls, [['docker', 'rm', '--force', 'run-1-0', 'run-1-1']]);
  const listed = linux(['list', 'run-1'], ['run-1-0', 'other-2-0', 'run-10-0', 'run-1-1']);
  assert.deepEqual(listed.calls, [['docker', 'ps', '--all', '--format', '{{.Names}}']]);
  assert.deepEqual([listed.status, listed.stdout], [0, 'run-1-0\nrun-1-1\n']);
  assert.deepEqual([linux(['list', 'run-1'], ['other-2-0']).status, linux(['list', 'run-1'], ['other-2-0']).stdout], [0, '']);
});

test('on Darwin each verb is one command of Apple container', (t) => {
  const darwin = (args: string[], names: string[] = []): Run => run(t, 'Darwin', 'container', args, names);
  assert.deepEqual([darwin(['program']).stdout, darwin(['program']).calls], ['container\n', []]);
  assert.deepEqual(darwin(['pull', IMAGE]).calls, [['container', 'image', 'pull', IMAGE]]);
  assert.deepEqual(darwin(['present', IMAGE]).calls, [['container', 'image', 'inspect', IMAGE]]);
  assert.deepEqual(darwin(['run', 'run-1-0', '--entrypoint', 'redis-cli', IMAGE, '-h', '192.0.2.1', 'ping']).calls, [
    ['container', 'run', '--rm', '--progress', 'none', '--name', 'run-1-0', '--entrypoint', 'redis-cli', IMAGE, '-h', '192.0.2.1', 'ping'],
  ]);
  assert.deepEqual(darwin(['delete', 'run-1-0']).calls, [['container', 'delete', '--force', 'run-1-0']]);
  const listed = darwin(['list', 'run-1'], ['run-1-0', 'other-2-0']);
  assert.deepEqual(listed.calls, [['container', 'ls', '--all', '--quiet']]);
  assert.deepEqual([listed.status, listed.stdout], [0, 'run-1-0\n']);
});

test('a container program that PATH does not hold fails every verb at once, with its name and what installs it', (t) => {
  for (const [system, program, install] of [['Linux', 'docker', 'Docker Engine'], ['Darwin', 'container', 'Apple container']] as const) {
    for (const args of [['program'], ['present', IMAGE], ['run', 'run-1-0', IMAGE]]) {
      const missing = run(t, system, undefined, args);
      assert.equal(missing.status, 1, `${system} ${args.join(' ')}`);
      assert.match(missing.stderr, new RegExp(`^scripts/container\\.sh: ${program} is not found on PATH: install ${install} `));
      assert.equal(missing.stdout, '');
    }
  }
});

test('a platform without a container program and a verb that the script does not know fail', (t) => {
  const other = run(t, 'FreeBSD', 'docker', ['program']);
  assert.equal(other.status, 1);
  assert.match(other.stderr, /^scripts\/container\.sh: no container program is named for FreeBSD: Darwin runs Apple container and Linux runs docker\n$/);
  assert.deepEqual(other.calls, []);
  for (const args of [[], ['start'], ['pull'], ['present'], ['run'], ['delete'], ['list'], ['list', 'a', 'b']]) {
    const unknown = run(t, 'Linux', 'docker', args);
    assert.equal(unknown.status, 2, args.join(' '));
    assert.match(unknown.stderr, /^usage: scripts\/container\.sh /);
    assert.deepEqual(unknown.calls, []);
  }
});
