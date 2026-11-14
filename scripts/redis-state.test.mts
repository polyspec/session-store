// The program of the state of the Redis service (scripts/redis-state.mts, SE-10.7), run through scripts/redis-state.sh
// against a stand-in of a Redis server that the test serves on a port that the system assigns: it answers the commands
// that the program sends from a map of its own, as Redis does, and records them, so the test needs no Redis server.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer, type AddressInfo, type Server } from 'node:net';
import { dirname, join } from 'node:path';
import { test, type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';

const program = join(dirname(fileURLToPath(import.meta.url)), 'redis-state.sh');

/** A key of the stand-in: the bytes of a string as latin1 text, or the items of a list. */
type Held = string | string[];

/** The words of each command of a RESP2 array of bulk strings in `buffer`, and the bytes that are left. */
function commands(buffer: Buffer): { words: string[][]; rest: Buffer } {
  const words: string[][] = [];
  let offset = 0;
  for (;;) {
    const start = offset;
    const line = buffer.indexOf('\r\n', offset);
    if (line < 0) return { words, rest: buffer.subarray(start) };
    const count = Number(buffer.toString('latin1', offset + 1, line));
    offset = line + 2;
    const command: string[] = [];
    for (let index = 0; index < count; index += 1) {
      const end = buffer.indexOf('\r\n', offset);
      if (end < 0) return { words, rest: buffer.subarray(start) };
      const length = Number(buffer.toString('latin1', offset + 1, end));
      if (buffer.length < end + 2 + length + 2) return { words, rest: buffer.subarray(start) };
      command.push(buffer.toString('latin1', end + 2, end + 2 + length));
      offset = end + 2 + length + 2;
    }
    words.push(command);
  }
}

const bulk = (text: string): string => `$${Buffer.byteLength(text, 'latin1')}\r\n${text}\r\n`;
const array = (items: string[]): string => `*${items.length}\r\n${items.map(bulk).join('')}`;
const WRONGTYPE = '-WRONGTYPE Operation against a key holding the wrong kind of value\r\n';

/** Whether `key` matches the pattern of SCAN `pattern`, a text with backslash escapes and a final `*`. */
function matches(pattern: string, key: string): boolean {
  assert.equal(pattern.endsWith('*'), true, pattern);
  const prefix = pattern.slice(0, -1).replace(/\\(.)/g, '$1');
  assert.doesNotMatch(pattern.slice(0, -1).replace(/\\./g, ''), /[*?[\]]/, pattern);
  return key.startsWith(prefix);
}

/** The reply of the stand-in to `words`, which changes `keys` as Redis does. */
function reply(keys: Map<string, Held>, words: string[], errors: Map<string, string>): string {
  const [name = '', ...args] = words;
  const error = errors.get(name);
  if (error !== undefined) return `-${error}\r\n`;
  const [key = ''] = args;
  const held = keys.get(key);
  switch (name) {
    case 'SELECT':
      return '+OK\r\n';
    case 'SCAN':
      return `*2\r\n${bulk('0')}${array([...keys.keys()].filter((found) => matches(args[2] ?? '', found)))}`;
    case 'TYPE':
      return `+${held === undefined ? 'none' : typeof held === 'string' ? 'string' : 'list'}\r\n`;
    case 'GET':
      return held === undefined ? '$-1\r\n' : typeof held === 'string' ? bulk(held) : WRONGTYPE;
    case 'LRANGE':
      return held === undefined ? '*0\r\n' : typeof held === 'string' ? WRONGTYPE : array(held);
    case 'DEL':
      return `:${keys.delete(key) ? 1 : 0}\r\n`;
    case 'SET':
      keys.set(key, args[1] ?? '');
      return '+OK\r\n';
    case 'RPUSH':
      if (typeof held === 'string') return WRONGTYPE;
      keys.set(key, [...(held ?? []), ...args.slice(1)]);
      return `:${(keys.get(key) as string[]).length}\r\n`;
    default:
      return `-ERR unknown command '${name}'\r\n`;
  }
}

/** A stand-in server with `keys`, which gives `errors` as the error reply of each command that it names. */
interface Stand {
  url: string;
  keys: Map<string, Held>;
  /** The commands that the stand-in received, each as its words. */
  received: string[][];
}

async function stand(t: TestContext, keys: Record<string, Held>, errors: Record<string, string> = {}): Promise<Stand> {
  const held = new Map(Object.entries(keys));
  const received: string[][] = [];
  const failures = new Map(Object.entries(errors));
  const server: Server = createServer((socket) => {
    let rest = Buffer.alloc(0);
    socket.on('data', (data: Buffer) => {
      const parsed = commands(Buffer.concat([rest, data]));
      rest = parsed.rest;
      for (const words of parsed.words) {
        received.push(words);
        socket.write(Buffer.from(reply(held, words, failures), 'latin1'));
      }
    });
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise<void>((resolve) => { server.close(() => resolve()); }));
  const { port } = server.address() as AddressInfo;
  return { url: `redis://127.0.0.1:${port}/3`, keys: held, received };
}

interface Run {
  status: number | null;
  stdout: string;
  stderr: string;
}

/** Runs the program with `args`, `input` on its standard input and SESSION_STORE_REDIS_URL `url`, and waits for its exit. */
function run(args: string[], input: string, url: string | undefined): Promise<Run> {
  return new Promise((resolve, reject) => {
    const env: Record<string, string> = { PATH: '/usr/bin:/bin' };
    if (url !== undefined) env['SESSION_STORE_REDIS_URL'] = url;
    const child = spawn(program, args, { env, stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (data: Buffer) => { stdout += data.toString('utf8'); });
    child.stderr.on('data', (data: Buffer) => { stderr += data.toString('utf8'); });
    child.on('error', reject);
    child.on('close', (status) => resolve({ status, stdout, stderr }));
    child.stdin.end(input);
  });
}

const SCOPE = 'python:redis_session.close:a_case:';
const OTHER = 'node:redis_session.close:a_case:';

test('setup makes the scope hold exactly the state of the input and leaves the keys of another scope', async (t) => {
  const made = await stand(t, { [`${SCOPE}old`]: 'x', [`${SCOPE}id`]: ['y'], [`${OTHER}id`]: 'z' });
  const ran = await run(['setup', SCOPE], JSON.stringify({ id: 'a:1:{s:1:"n";i:0;}', id_LOCK: { list: ['another holder', 'ÿ'] } }), made.url);
  assert.deepEqual(ran, { status: 0, stdout: '', stderr: '' });
  assert.deepEqual(Object.fromEntries(made.keys), {
    [`${OTHER}id`]: 'z',
    [`${SCOPE}id`]: 'a:1:{s:1:"n";i:0;}',
    [`${SCOPE}id_LOCK`]: ['another holder', 'ÿ'],
  });
  // The database of the URL is selected first, and the byte U+00FF is sent as one byte.
  assert.deepEqual(made.received[0], ['SELECT', '3']);
});

test('change writes a key in place of a key of another type and removes a key that is null', async (t) => {
  const made = await stand(t, { [`${SCOPE}id`]: 'r', [`${SCOPE}id_LOCK`]: 'mine', [`${SCOPE}gone`]: 'g' });
  const ran = await run(['change', SCOPE], JSON.stringify({ id_LOCK: { list: ['another holder'] }, gone: null }), made.url);
  assert.deepEqual(ran, { status: 0, stdout: '', stderr: '' });
  assert.deepEqual(Object.fromEntries(made.keys), { [`${SCOPE}id`]: 'r', [`${SCOPE}id_LOCK`]: ['another holder'] });
});

test('check passes when the scope holds the state of the input, and fails with each key that differs', async (t) => {
  const made = await stand(t, { [`${SCOPE}id`]: 'r', [`${SCOPE}id_LOCK`]: ['another holder'], [`${OTHER}id`]: 'z' });
  assert.deepEqual(await run(['check', SCOPE], JSON.stringify({ id: 'r', id_LOCK: { list: ['another holder'] } }), made.url), { status: 0, stdout: '', stderr: '' });
  const differs = await run(['check', SCOPE], JSON.stringify({ id: 's', other: 'o' }), made.url);
  assert.deepEqual(differs, {
    status: 1,
    stdout: [
      `the scope ${SCOPE} holds another state:`,
      `${SCOPE}id: expected "s", found "r"`,
      `${SCOPE}id_LOCK: expected no key, found {"list":["another holder"]}`,
      `${SCOPE}other: expected "o", found no key`,
      '',
    ].join('\n'),
    stderr: '',
  });
  assert.equal((await run(['check', SCOPE], '{}', made.url)).status, 1);
});

test('a scope with a character of a pattern is matched as text', async (t) => {
  const made = await stand(t, { 'a*[b]?:k': 'v' });
  assert.deepEqual(await run(['check', 'a*[b]?:'], JSON.stringify({ k: 'v' }), made.url), { status: 0, stdout: '', stderr: '' });
  assert.deepEqual(made.received.find((words) => words[0] === 'SCAN')?.[3], 'a\\*\\[b\\]\\?:*');
});

test('an error reply fails the program with the command and the reply', async (t) => {
  const made = await stand(t, {}, { SET: 'OOM command not allowed' });
  assert.deepEqual(await run(['setup', SCOPE], JSON.stringify({ id: 'r' }), made.url), {
    status: 1,
    stdout: '',
    stderr: 'scripts/redis-state.mts: SET gets the error reply OOM command not allowed\n',
  });
});

test('an input or an address of another form fails before any command, and other arguments fail with the usage', async (t) => {
  const made = await stand(t, {});
  const refused = [
    [['setup', SCOPE], '[]', made.url, /is no JSON object/],
    [['setup', SCOPE], 'x', made.url, /is no JSON/],
    [['setup', SCOPE], JSON.stringify({ id: 1 }), made.url, /id is neither a text nor/],
    [['setup', SCOPE], JSON.stringify({ id: null }), made.url, /id is neither a text nor .* at least one item: null/],
    [['setup', SCOPE], JSON.stringify({ id: { list: [] } }), made.url, /id is neither a text nor/],
    [['setup', SCOPE], JSON.stringify({ id: 'Ā' }), made.url, /holds a character above U\+00FF/],
    [['setup', SCOPE], '{}', undefined, /SESSION_STORE_REDIS_URL is not set/],
    [['setup', SCOPE], '{}', 'rediss://h:1', /is no URL redis:\/\/<host>:<port>/],
  ] as const;
  for (const [args, input, url, message] of refused) {
    const ran = await run([...args], input, url);
    assert.equal(ran.status, 1, `${input} ${url}`);
    assert.match(ran.stderr, message);
  }
  assert.deepEqual(made.received, []);
  for (const args of [[], ['setup'], ['clear', SCOPE], ['setup', SCOPE, 'more'], ['check', '']]) {
    assert.equal((await run(args, '{}', made.url)).status, 2, args.join(' '));
  }
});
