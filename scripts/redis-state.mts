// The program of the state of the Redis service of the test run (SE-10.7, binding GN-13j), which a generated binding
// test starts through scripts/redis-state.sh:
//
//   scripts/redis-state.sh setup|change|check <scope>     with a JSON object on the standard input
//
// The object maps the name of a key after the scope to its value: a text, whose characters U+0000 to U+00FF are the
// bytes of a string (SE-10.2), or {"list": [<text>, ...]}, a list of at least one such item; in change, null removes
// the key.
//
//   setup   removes every key whose name starts with the scope and writes each key of the object, so that the scope
//           holds exactly that state;
//   change  writes each key of the object, in place of a key of another type, and removes each key that is null, as
//           another client of the server does while a session lives;
//   check   reads every key of the scope and exits with 0 when they are the keys of the object with those values; else
//           it prints each key that differs, with its expected and its found value, and exits with 1.
//
// The server is the URL redis://<host>:<port>[/<database>] of SESSION_STORE_REDIS_URL. The program speaks the protocol
// of Redis (RESP2) itself over a socket of node:net, so it needs no package: it sends one command and reads its reply
// before the next and waits for no time. An error reply, a connection that fails or closes, and an input of another
// form fail it with the status 1 and the reason.
import { connect, type Socket } from 'node:net';
import { readFileSync } from 'node:fs';

const NAME = 'scripts/redis-state.mts';
const VARIABLE = 'SESSION_STORE_REDIS_URL';

/** A value of the state: the bytes of a string, as latin1 text, or the items of a list. */
type Value = string | { list: string[] };

/** A reply of Redis. */
type Reply = string | number | Buffer | null | Reply[] | { error: string };

function fail(message: string): never {
  process.stderr.write(`${NAME}: ${message}\n`);
  process.exit(1);
}

/** The text of `value` when it is a text of bytes (U+0000 to U+00FF, SE-10.2). */
function bytesOf(value: unknown, where: string): string {
  if (typeof value !== 'string') fail(`${where} is no text: ${JSON.stringify(value)}`);
  if (/[^\u0000-ÿ]/.test(value)) fail(`${where} holds a character above U+00FF: ${JSON.stringify(value)}`);
  return value;
}

/** A value of the input; null only where `removable`. */
function valueOf(value: unknown, where: string, removable: boolean): Value | null {
  if (value === null && removable) return null;
  if (typeof value === 'string') return bytesOf(value, where);
  if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
    const entries = Object.entries(value);
    const list = (value as { list?: unknown }).list;
    if (entries.length === 1 && Array.isArray(list) && list.length > 0) {
      return { list: list.map((item, index) => bytesOf(item, `${where}.list[${index}]`)) };
    }
  }
  fail(`${where} is neither a text nor {"list": [<text>, ...]} with at least one item${removable ? ' nor null' : ''}: ${JSON.stringify(value)}`);
}

/** A reply that is complete in `buffer` from `start`, with the offset after it, or undefined while it is incomplete. */
function parse(buffer: Buffer, start: number): { reply: Reply; end: number } | undefined {
  const line = buffer.indexOf('\r\n', start);
  if (line < 0) return undefined;
  const head = buffer.toString('latin1', start + 1, line);
  const next = line + 2;
  switch (String.fromCharCode(buffer[start] ?? 0)) {
    case '+':
      return { reply: head, end: next };
    case '-':
      return { reply: { error: head }, end: next };
    case ':':
      return { reply: Number(head), end: next };
    case '$': {
      const length = Number(head);
      if (length < 0) return { reply: null, end: next };
      if (buffer.length < next + length + 2) return undefined;
      return { reply: buffer.subarray(next, next + length), end: next + length + 2 };
    }
    case '*': {
      const count = Number(head);
      if (count < 0) return { reply: null, end: next };
      const items: Reply[] = [];
      let offset = next;
      for (let index = 0; index < count; index += 1) {
        const item = parse(buffer, offset);
        if (item === undefined) return undefined;
        items.push(item.reply);
        offset = item.end;
      }
      return { reply: items, end: offset };
    }
    default:
      fail(`the server sends a reply that RESP2 does not have: ${JSON.stringify(buffer.toString('latin1', start, line))}`);
  }
}

/** A connection that sends one command and waits for its reply before the next. */
class Connection {
  private buffer = Buffer.alloc(0);
  private waiting: ((reply: Reply) => void) | undefined;

  private readonly socket: Socket;

  private constructor(socket: Socket) {
    this.socket = socket;
    socket.on('data', (data: Buffer) => {
      this.buffer = Buffer.concat([this.buffer, data]);
      const parsed = parse(this.buffer, 0);
      if (parsed === undefined) return;
      this.buffer = this.buffer.subarray(parsed.end);
      const waiting = this.waiting;
      this.waiting = undefined;
      if (waiting === undefined) fail('the server sends a reply to no command');
      waiting(parsed.reply);
    });
    socket.on('close', () => {
      if (this.waiting !== undefined) fail('the server closes the connection before it replies');
    });
  }

  static open(host: string, port: number): Promise<Connection> {
    return new Promise((resolve) => {
      const socket = connect({ host, port });
      socket.once('error', (error) => fail(`the connection to ${host}:${port} fails: ${error.message}`));
      socket.once('connect', () => resolve(new Connection(socket)));
    });
  }

  /** The reply of the command `words`; an error reply fails the program. */
  async send(...words: string[]): Promise<Reply> {
    const parts = [`*${words.length}\r\n`];
    for (const word of words) parts.push(`$${Buffer.byteLength(word, 'latin1')}\r\n${word}\r\n`);
    const reply = await new Promise<Reply>((resolve) => {
      this.waiting = resolve;
      this.socket.write(Buffer.from(parts.join(''), 'latin1'));
    });
    if (typeof reply === 'object' && reply !== null && !Buffer.isBuffer(reply) && !Array.isArray(reply)) {
      fail(`${words[0]} gets the error reply ${reply.error}`);
    }
    return reply;
  }

  close(): void {
    this.socket.end();
  }
}

/** The pattern of SCAN that matches the keys that start with `scope`. */
function patternOf(scope: string): string {
  return `${scope.replace(/[*?[\]\\]/g, (character) => `\\${character}`)}*`;
}

/** The names of the keys of the scope, after the scope. */
async function keysOf(connection: Connection, scope: string): Promise<string[]> {
  const found = new Set<string>();
  let cursor = '0';
  do {
    const reply = await connection.send('SCAN', cursor, 'MATCH', patternOf(scope), 'COUNT', '1000');
    if (!Array.isArray(reply) || reply.length !== 2 || !Buffer.isBuffer(reply[0]) || !Array.isArray(reply[1])) fail('SCAN gets a reply that is no cursor and list');
    cursor = reply[0].toString('latin1');
    for (const key of reply[1]) {
      if (!Buffer.isBuffer(key)) fail('SCAN gives a key that is no bulk string');
      found.add(key.toString('latin1').slice(scope.length));
    }
  } while (cursor !== '0');
  return [...found].sort();
}

/** The value of a key, or the name of its type when it is neither a string nor a list. */
async function read(connection: Connection, key: string): Promise<Value | { type: string } | null> {
  const type = await connection.send('TYPE', key);
  if (type === 'none') return null;
  if (type === 'string') {
    const reply = await connection.send('GET', key);
    return Buffer.isBuffer(reply) ? reply.toString('latin1') : null;
  }
  if (type === 'list') {
    const reply = await connection.send('LRANGE', key, '0', '-1');
    if (!Array.isArray(reply)) fail(`LRANGE of ${key} gets no list`);
    return { list: reply.map((item) => (Buffer.isBuffer(item) ? item.toString('latin1') : String(item))) };
  }
  return { type: String(type) };
}

/** Writes `value` into `key`, in place of what the key held. */
async function write(connection: Connection, key: string, value: Value | null): Promise<void> {
  await connection.send('DEL', key);
  if (value === null) return;
  if (typeof value === 'string') await connection.send('SET', key, value);
  else await connection.send('RPUSH', key, ...value.list);
}

const [step, scope, ...rest] = process.argv.slice(2);
if ((step !== 'setup' && step !== 'change' && step !== 'check') || scope === undefined || scope === '' || rest.length > 0) {
  process.stderr.write(`usage: ${NAME} setup|change|check <scope>, with a JSON object on the standard input\n`);
  process.exit(2);
}
let input: unknown;
try {
  input = JSON.parse(readFileSync(0, 'utf8'));
} catch (error) {
  fail(`the standard input is no JSON: ${(error as Error).message}`);
}
if (typeof input !== 'object' || input === null || Array.isArray(input)) fail(`the standard input is no JSON object: ${JSON.stringify(input)}`);
const state = Object.entries(input).map(([key, value]) => [bytesOf(key, 'a key'), valueOf(value, key, step === 'change')] as const);

const address = process.env[VARIABLE] ?? '';
if (address === '') fail(`${VARIABLE} is not set: the test run that starts the Redis service sets it (make test-python, test-node, test-php, test-go)`);
let url: URL;
try {
  url = new URL(address);
} catch {
  fail(`${VARIABLE} is no URL: ${address}`);
}
if (url.protocol !== 'redis:' || url.hostname === '' || url.port === '' || url.username !== '' || url.password !== '' || url.search !== '') {
  fail(`${VARIABLE} is no URL redis://<host>:<port>[/<database>]: ${address}`);
}
const database = url.pathname.replace(/^\//, '');
if (database !== '' && !/^[0-9]+$/.test(database)) fail(`${VARIABLE} names a database that is no number: ${address}`);

const connection = await Connection.open(url.hostname.replace(/^\[(.*)\]$/, '$1'), Number(url.port));
if (database !== '') await connection.send('SELECT', database);
if (step === 'setup') {
  for (const key of await keysOf(connection, scope)) await connection.send('DEL', `${scope}${key}`);
}
if (step === 'setup' || step === 'change') {
  for (const [key, value] of state) await write(connection, `${scope}${key}`, value);
  connection.close();
} else {
  const expected = new Map(state);
  const differences: string[] = [];
  const found = await keysOf(connection, scope);
  for (const key of [...new Set([...found, ...expected.keys()])].sort()) {
    const value = found.includes(key) ? await read(connection, `${scope}${key}`) : null;
    const wanted = expected.get(key) ?? null;
    if (JSON.stringify(value) !== JSON.stringify(wanted)) {
      differences.push(`${scope}${key}: expected ${wanted === null ? 'no key' : JSON.stringify(wanted)}, found ${value === null ? 'no key' : JSON.stringify(value)}`);
    }
  }
  connection.close();
  if (differences.length > 0) {
    process.stdout.write(`the scope ${scope} holds another state:\n${differences.join('\n')}\n`);
    process.exit(1);
  }
}
