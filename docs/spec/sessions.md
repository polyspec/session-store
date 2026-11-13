# session-store: rules

This document is the specification of session-store. A rule is a list item `**SE-n**` (the store and its record, crate
`polyspec-session-store-core`), `**SF-n**` (the store of files, crate `polyspec-session-store-file`) or `**SR-n**` (the
store of Redis, crate `polyspec-session-store-redis`); a statement of a rule has the anchor `SE-n.m`, and a test names
the statement that it checks. A PHP value and its serialize form are those of php-serialization
(`polyspec-php-serialization`, rules RQ-n). phpredis is the PHP extension phpredis 6.3.0; "PHP" is PHP 8.4.

## Scope

- **SE-1** <a id="SE-1.1"></a> session-store stores the values of browser sessions: it creates and accepts session
  identifiers, opens, locks, reads, writes and closes the session of one request in that order, renews a session under
  a new identifier, ends a session after its lifetime and removes ended sessions. <a id="SE-1.2"></a> It does not know
  what a session holds: a key is a text and a value is a PHP value (RQ-008), and no key and no value has a meaning for
  the library. Which keys an application uses, what their values mean and how the values relate to other value types
  (such as the data model of a template language) are rules of the application, not of this library.
  <a id="SE-1.3"></a> Every rule is in the Rust crates of `packages/core/rust`, `packages/file/rust` and
  `packages/redis/rust`; a binding converts types and holds no rule. The crate `polyspec-session-store` of
  `packages/rust` is the library: it gives the items of the three crates again, the stores under the features `file` and
  `redis`, which it builds by default, and its package holds the one interface contract of the bindings, so that every
  language has one package of session-store. <a id="SE-1.4"></a> `polyspec-session-store-core` does no I/O: it reads no
  file, no clock, no environment and no network. Its only outside source is the random source of the
  operating system (SE-5.3), because an identifier that another party can predict names another browser's session.
  <a id="SE-1.5"></a> The three crates depend on `polyspec-php-serialization` and `polyspec-binding-core` and on no
  library of another value type.

## The store interface

- **SE-2** <a id="SE-2.1"></a> A store holds the values of one session for one request. It has four operations: `get`
  of a key returns the value that the key holds, or nothing when the key holds no value; `set` of a key and a value
  makes the key hold the value; `remove` of a key makes the key hold no value; `renew` moves the values to a new session
  identifier (SE-7). <a id="SE-2.2"></a> `set` of a key that holds a value replaces the value, and `set` of a key that
  holds none adds it. <a id="SE-2.3"></a> `remove` of a key that holds no value succeeds and changes nothing.
  <a id="SE-2.4"></a> A value is a PHP value whose top value is no reference (RQ-008); `get` gives a value that the
  caller owns, so a change of it does not change the store, and `set` takes a copy. <a id="SE-2.5"></a> A key is a text
  (UTF-8). Two keys name one value when PHP stores them as one array key: a text in the canonical form of a 64-bit
  integer names that integer (RQ-004), so `"5"` and the integer key 5 of a record are one key.
- **SE-3** <a id="SE-3.1"></a> The store interface is a public API, so an application may implement a store in its own
  language. A binding wraps such a store as an implementation of the Rust trait `Store` and calls its four members
  (binding design U-3). <a id="SE-3.2"></a> An exception that such a store raises is not an error of this library: it
  stops the operation, goes through the core unchanged and reaches the caller as the same object; the library neither
  wraps it nor changes it. A store that Rust code implements returns its own error the same way
  (`Failure::Callback` of binding-core), and the operation gives it back to that Rust caller unchanged. <a id="SE-3.3"></a> Such a store must follow SE-2. Its locking (SE-8), its sharing with other
  processes and its interoperation with other servers are the responsibility of the application, and the library checks
  none of them. <a id="SE-3.4"></a> In a language whose null is both the absence of a value and the PHP value null (PHP,
  Python, Node), `get` of a key that holds no value and of a key that holds null give the same language value; the Rust
  API tells them apart (`Option`), and a record keeps a key that holds null (SE-6.6).
- **SE-4** <a id="SE-4.1"></a> The memory store keeps the values of one session in the memory of one process, for tests
  and for an application whose sessions live as long as its process. <a id="SE-4.2"></a> It keeps the order of its keys
  as SE-2.2 states. <a id="SE-4.3"></a> Its `renew` changes no value and counts the renewals, which `renewals` returns,
  because the memory store has no identifier.

## Identifiers

- **SE-5** <a id="SE-5.1"></a> A session identifier is a text of 22 to 256 characters, each of `A`–`Z`, `a`–`z`, `0`–`9`,
  `,` and `-`. The characters and the lengths are those of an identifier that PHP creates for its session handlers
  (`session.sid_length` from 22 to 256), so every identifier that PHP created is accepted. <a id="SE-5.2"></a>
  `accept` takes the bytes of a session cookie value and gives the identifier, or nothing for any bytes that are not an
  identifier, which is the only reason of its absence. A value that is not an identifier names no session and fails
  nothing, because a cookie is input of the browser. <a id="SE-5.3"></a> A new identifier is 32 bytes of the random
  source of the operating system (`getrandom`), written as 64 lowercase hexadecimal digits. <a id="SE-5.4"></a> When the
  random source fails, the operation fails with the kind `random`; no other source replaces it.

## The record

- **SE-6** <a id="SE-6.1"></a> The record of a session is the bytes of its values: the serialize form (RQ-001) of one
  PHP array whose keys are the keys of the session and whose values are their values. It is the form `php_serialize`
  of the PHP session module. <a id="SE-6.2"></a> A record without bytes is a session without values.
  <a id="SE-6.3"></a> Bytes that unserialize refuses fail with the kind `record`, whose source is the error of
  php-serialization with its kind and offset. <a id="SE-6.4"></a> A record whose top value is no array fails with the
  kind `record_not_array`; the message names the identifier: `the session of <id> holds no array of values`.
  <a id="SE-6.5"></a> `get` of a key whose element is a reference gives the value that the reference holds, because a
  value of a store is no reference (SE-2.4). An object or a reference that the value shares with other elements of the
  record is a copy in the given value. <a id="SE-6.6"></a> A record that a request writes holds every key in the order of
  the array: a key that the request did not set holds its value as it was read, with the objects and references that it
  shares with other such keys, and a key that holds null is kept. A key of the record whose string is not UTF-8 is kept
  as it was, and no text names it (SE-2.5). <a id="SE-6.7"></a> A set key holds the value that the request gave, and
  the objects and references of that value are shared only within it. <a id="SE-6.8"></a> Values that have no record
  that php-serialization writes, such as a value nested so deep that the record passes the depth of RQ-006, fail the
  `finish` of the session with the kind `record_write`, whose source is the error of php-serialization.

## The session of a request

- **SE-7** <a id="SE-7.1"></a> The record session is the store of one request of a store that keeps records (SE-6),
  such as the stores of files and of Redis; it does no I/O. Its store opens it with what the store found: the identifier
  and the record of an existing session, or nothing. <a id="SE-7.2"></a> The session starts at the first `get`, `set`,
  `remove` or `renew`, so a request that never uses it creates no session. Starting an existing session decodes its
  record (SE-6), and a record that fails fails that operation and every later one with the same kind. Starting without
  an existing session creates a new identifier (SE-5.3) and a session without values. <a id="SE-7.3"></a> `renew` starts
  the session and gives it a new identifier; the identifier that the session had when it was found is the one that the
  close removes. A second `renew` changes the identifier again and still removes only the found one.
  <a id="SE-7.4"></a> `finish` ends the record session and gives what its store writes: `unused` for a session that
  never started; `unchanged` with the identifier for an existing session whose record of the values equals the record
  that was read, byte for byte, and whose identifier did not change; and `write` with the identifier, the record, whether
  the identifier is new and the identifier to remove (for a renewal) otherwise. <a id="SE-7.5"></a> After `finish` the
  record session is gone; a binding refuses another call with the kind `consumed` (binding design E-1a).
  <a id="SE-7.6"></a> A store that writes a new identifier gives it to its caller, which sets it as the session cookie;
  the name and the attributes of that cookie are rules of the application.
- **SE-8** <a id="SE-8.1"></a> The requests of one session run one after another: a store that keeps records holds a
  lock of the session from the opening of the session of a request to its closing (SF-3, SR-3), so no write of one
  request is lost by the write of another. <a id="SE-8.2"></a> A request that opens no session holds no lock.

## Errors

- **SE-9** <a id="SE-9.1"></a> Every failure of the library is an error of a crate's error enum with a kind; a caller
  tells failures apart by the kind, and a message is a rule only where a rule states it (SE-6.4). <a id="SE-9.2"></a>
  The kinds of `polyspec-session-store-core`:

  | Kind | Status | Category | Rule |
  |---|---|---|---|
  | `random` | 500 | `io` | SE-5.4 |
  | `record` | 500 | `store` | SE-6.3 |
  | `record_not_array` | 500 | `store` | SE-6.4 |
  | `record_write` | 500 | `store` | SE-6.8 |
  | `internal` | 500 | `internal` | an invariant of the implementation fails; it is a defect |

  The errors of `record` and `record_write` carry the offset of the error of php-serialization, or none (`offset`).
  <a id="SE-9.3"></a> The status is that of the request whose session fails: a session that the server cannot read or
  write fails the request as an error of the server. <a id="SE-9.4"></a> The kinds of the stores (SF-9, SR-9) are
  variants of the same error enum, `Error` of `polyspec-session-store-core`, so that one kind interface and one table of
  classes hold every failure of the library, and a failure of the record that a store meets keeps its kind.
  <a id="SE-9.5"></a> The category `internal` holds only the kind `internal`, a defect of the implementation. A
  failure that the environment of the process causes without a defect is of the category `io`: the random source of
  the operating system fails (`random`, SE-5.4), or another process created the file of a new identifier
  (`identifier_taken`, SF-4.4). A caller that tells a defect from a failure of the environment by the class of the
  error is then right for both.

## The store of files

- **SF-1** <a id="SF-1.1"></a> The store of files keeps each session in a file of one directory, so the server processes
  that share the directory share its sessions. <a id="SF-1.2"></a> The directory is an absolute path of an existing
  directory; another path fails the making of the store with the kind `invalid_directory`, and the error names the
  path. A `session.save_path` of PHP in the form `N;/path` or `N;MODE;/path` (subdirectories by the first characters of
  the identifier) is not an absolute path and fails so: the store keeps every file in its directory.
  <a id="SF-1.3"></a> The lifetime is a positive number of seconds, 1440 when the caller states none, the default of
  `session.gc_maxlifetime` of PHP; 0 fails with the kind `invalid_lifetime`.
- **SF-2** <a id="SF-2.1"></a> A session is the file `sess_<id>` of the directory, whose bytes are its record (SE-6)
  and whose mode is 0600 when the store creates it: the file name, the record and the mode of the files handler of the
  PHP session module. <a id="SF-2.2"></a> A file without bytes is a session without values (SE-6.2). <a id="SF-2.3"></a>
  A session whose file was last modified more than the lifetime ago has ended: it names no session.
  <a id="SF-2.4"></a> The store opens every file with `O_NOFOLLOW` and `O_CLOEXEC`, so a symbolic link in the place of a
  session file fails the open and an executed child process does not inherit the file and its lock.
- **SF-3** <a id="SF-3.1"></a> `open` takes the bytes of the session cookie and gives the record session of the
  request (SE-7). Bytes that are not an identifier (SE-5.2) give a record session without an existing session and
  without a lock. <a id="SF-3.2"></a> The requests of one identifier in one process wait in a queue of that process,
  first come first served, before they try the lock, so that one thread of a process waits on the lock of a session.
  <a id="SF-3.3"></a> In its turn a request opens the file without creating it; a missing file gives a record session
  without an existing session, and the request leaves its turn. <a id="SF-3.4"></a> The request takes `flock` with
  `LOCK_EX` on the file and blocks until it holds it, on its own thread; the operating system orders the waiting
  processes, and the library has no retry and no pause. <a id="SF-3.5"></a> When it holds the lock, it compares the
  file that it locked with the file that the path names (device and inode); when the path names another file or none,
  the holder before it removed or replaced the file (SF-4.4), so it releases the lock and opens again from SF-3.3.
  <a id="SF-3.6"></a> A file older than the lifetime (SF-2.3) gives a record session without an existing session; the
  request releases the lock and its turn. <a id="SF-3.7"></a> Otherwise it reads the file from its start to its end
  and keeps the lock and its turn until `close`. <a id="SF-3.8"></a> A caller that gave up waiting (its `cancel`, such as a request whose
  connection closed) fails with the kind `cancelled` and lets the next request of the session go on. A request that
  waits in the queue is woken by the cancellation at once and leaves the queue. A request that waits in `flock` cannot
  be woken, because no thread can interrupt a blocking `flock`: when it gets the lock, it sees the cancellation,
  releases the lock and its turn at once and fails.
- **SF-4** <a id="SF-4.1"></a> `close` takes the record session of the request and writes what `finish` gives (SE-7.4).
  <a id="SF-4.2"></a> `unused` writes nothing. `unchanged` keeps the bytes and sets the modification time of the file
  to now, so the session lives a lifetime from its last request. <a id="SF-4.3"></a> `write` of an existing identifier
  writes the record over the file in place from its start under the lock and cuts the file to the length of the record,
  as the files handler of PHP does; a process that ends during the write can leave a record that the next request
  cannot read (SE-6.3). <a id="SF-4.4"></a> `write` of a new identifier creates its file with `O_CREAT | O_EXCL` and
  the mode 0600 and writes the record; no other process knows the identifier, so the file needs no lock. A renewal
  creates the new file before it removes the file of the old identifier, so that a failure leaves the session under one
  of the two. A file that already has the new identifier fails with the kind `identifier_taken`. <a id="SF-4.5"></a>
  `close` releases the lock and the turn also when a write fails, and gives the new identifier, or nothing.
- **SF-5** <a id="SF-5.1"></a> `collect` removes the files of ended sessions (SF-2.3) and gives how many it removed.
  <a id="SF-5.2"></a> For each file `sess_<id>` of the directory whose `<id>` is an identifier, it tries the lock without
  waiting (`LOCK_NB`); a file whose lock a request holds is skipped, and a file that is still within its lifetime once
  locked, or that the path no longer names, is kept. <a id="SF-5.3"></a> The library never calls `collect` by itself:
  the application calls it when it chooses, such as from a scheduled task, and the store does not delete a session when
  a request opens it.
- **SF-6** <a id="SF-6.1"></a> The PHP files handler and this store share the file name, the record, the mode 0600 and
  `flock(LOCK_EX)`, so that a PHP process that uses the native session module with `session.serialize_handler`
  `php_serialize` and this store can read each other's sessions. They differ, and these differences are not repaired:
  - the native handler reads a file without comparing its modification time with the lifetime, so it reads an ended
    session that its garbage collection has not removed;
  - it opens a file with `O_CREAT`, so it creates a file for an identifier that names no session;
  - its garbage collection removes files with `unlink` without the lock, so it can remove the file of a session that a
    request holds;
  - it does not check after the lock that the file was removed or replaced;
  - `session_regenerate_id(true)` removes the old file before it writes the new one;
  - it refuses a file whose owner is not the user of the process, which this store does not check.
- **SF-7** <a id="SF-7.1"></a> `flock` holds until every descriptor of the open file is closed, so a child of `fork`
  without `exec` that inherits the descriptor of a held session holds the session until it ends or closes the
  descriptor; `O_CLOEXEC` (SF-2.4) covers the children that execute another program.
- **SF-8** <a id="SF-8.1"></a> The waits (plan B-5): `open` waits on another process (`ProcessWait`), and `new`,
  `close` and `collect` do short blocking I/O (`Io`). A Rust function that blocks takes its token as its first argument.
  <a id="SF-8.2"></a> The lifetime is an unsigned integer of the interface contract (`uint`), so a binding refuses a
  value that is no integer of 0 or more before the store runs: a negative or fractional number with the kind
  `invalid_number` and a text or a boolean with the kind `invalid_argument_type` (binding design T-1, E-1a). Such cases
  of `contracts/file-sessions.json` carry `binding`, and the bindings run them; the Rust tests show them as ignored.
- **SF-9** <a id="SF-9.1"></a> The kinds of `polyspec-session-store-file`:

  | Kind | Status | Category | Rule |
  |---|---|---|---|
  | `invalid_directory` | none | `value` | SF-1.2 |
  | `invalid_lifetime` | none | `value` | SF-1.3 |
  | `cancelled` | 500 | `io` | SF-3.8 |
  | `identifier_taken` | 500 | `io` | SF-4.4 |
  | `not_found`, `permission_denied`, `cross_device`, `storage_full`, `io` | 500 | `io` | an operation of the file system fails; the error carries the `errno` |
  | the kinds of SE-9.2 | | | the core |

  <a id="SF-9.2"></a> The kind of a failure of the file system follows its `errno`: `ENOENT` is `not_found`, `EACCES`
  and `EPERM` are `permission_denied`, `EXDEV` is `cross_device`, `ENOSPC` and `EDQUOT` are `storage_full`, and every
  other is `io`. The error names the operation and the path. <a id="SF-9.3"></a> `invalid_directory` and
  `invalid_lifetime` are errors of the call that makes the store, not of a request, so they have no status.

## The store of Redis

- **SR-1** <a id="SR-1.1"></a> The store of Redis keeps each session in a key of a Redis server, with the keys, the lock
  and the record of the session handler of phpredis, so the server processes that share the Redis server share its
  sessions with each other and with PHP processes that use phpredis as SR-7 states. <a id="SR-1.2"></a> The options and
  their defaults:

  | Option | Meaning | Default | Refused |
  |---|---|---|---|
  | `lifetime` | seconds after the last request of a session until it ends | 1440 (`session.gc_maxlifetime`) | 0, kind `invalid_lifetime` |
  | `prefix` | the prefix of the keys, the `prefix` of `session.save_path` of phpredis | `PHPREDIS_SESSION:` | none |
  | `lock_expire` | seconds until a lock expires, `redis.session.lock_expire` | none: the caller states it | 0 and a number of seconds whose milliseconds pass 2^64 − 1, kind `invalid_lock_expire` |
  | `lock_retries` | how many more times a request tries a taken lock, `redis.session.lock_retries` | 100 | none |
  | `lock_wait` | microseconds between two tries, `redis.session.lock_wait_time` | 20000 | none |

  <a id="SR-1.3"></a> Every process of one Redis server gives the options the same values, and `lock_expire` is longer
  than any request. `lock_expire` has no default, because no value is longer than every request of every application
  and phpredis has none either (its 0 takes `max_execution_time`); the caller states it.
- **SR-2** <a id="SR-2.1"></a> A session is the key `<prefix><id>`, whose bytes are its record (SE-6) and which expires
  after the lifetime. Its lock is the key `<prefix><id>_LOCK`. <a id="SR-2.2"></a> A request takes the lock with a
  secret of its own: 16 bytes of the random source of the operating system as 32 lowercase hexadecimal digits, so that
  no request, of this process or another, holds the secret of another lock. <a id="SR-2.3"></a> The release script is
  `if redis.call("get",KEYS[1]) == ARGV[1] then return redis.call("del",KEYS[1]) else return 0 end`, which deletes the
  lock only for its holder.
- **SR-3** <a id="SR-3.1"></a> The protocol is a state machine without I/O: it gives the command to send or the time
  to wait, and takes the reply; the connection (SR-6) sends and waits. Its commands, in this order, are the interface
  with phpredis and are checked by the command records of `contracts/redis-sessions.json`. <a id="SR-3.2"></a> Opening:
  bytes that are not an identifier give a record session without an existing session and send nothing. Otherwise the
  requests of one identifier in one process wait in a queue of that process (as SF-3.2), and the request sends
  `EXISTS <key>`; a reply 0 gives a record session without an existing session. <a id="SR-3.3"></a> Then it sends
  `SET <key>_LOCK <secret> NX PX <lock_expire × 1000>`; the reply `OK` takes the lock, and a nil reply means that
  another holds it: the request waits `lock_wait` and tries again, `lock_retries` more times in all, and then fails with
  the kind `lock_timeout`, without a release, since it holds nothing. <a id="SR-3.4"></a> With the lock it sends
  `GET <key>`: a bulk string is the record and gives the record session of an existing session, whose lock and turn the
  request keeps until it closes; a nil reply, a key that expired or was removed since `EXISTS`, gives a record session
  without an existing session after the release (SR-2.3).
- **SR-4** <a id="SR-4.1"></a> Closing writes what `finish` gives (SE-7.4). `unused` sends only the release of a held
  lock. <a id="SR-4.2"></a> `unchanged` sends `EXPIRE <key> <lifetime>` without comparing the lock, because it writes no
  record, so a long request that only reads does not fail when its lock expired; then the release.
  <a id="SR-4.3"></a> `write` of an existing identifier first sends `GET <key>_LOCK`: a reply other than the secret of
  the request means that the lock expired and another may hold it, so the request writes nothing, sends the release and
  fails with the kind `lock_lost`. <a id="SR-4.4"></a> Then it sends `SETEX <key> <lifetime> <record>`; a renewal writes
  the new key and then sends `DEL <old key>`; then the release. A request that holds a lock compares it before it
  writes anything, also when it writes a new identifier of a renewal; a request that found no session holds no lock
  and sends no `GET` of a lock. <a id="SR-4.5"></a> A failure after the lock was taken still sends the release before the error reaches the
  caller, and the request leaves its turn in every case.
- **SR-5** <a id="SR-5.1"></a> A reply of another type than the command gives (an integer for `EXISTS`, `OK` or nil for
  `SET`, a bulk string or nil for `GET`, an integer for `EXPIRE`, `DEL` and the release, `OK` for `SETEX`) fails with
  the kind `reply`, after the release of a held lock; a bulk string is never read as text, so the bytes of a record stay
  bytes. <a id="SR-5.2"></a> An error reply of Redis fails with the kind `redis`, which names the command.
- **SR-6** <a id="SR-6.1"></a> The connection uses the crate `redis` (redis-rs) on one tokio runtime of the process. The
  application gives a connection setting, not a client: a server, Sentinel or Cluster, each with its addresses, TLS and
  an ACL user and password. <a id="SR-6.2"></a> With Sentinel, a request that sends a command asks Sentinel for the
  primary of its service before its first command, also when a connection to the former primary is open, because a
  former primary accepts writes for some seconds after a promotion. <a id="SR-6.3"></a> With Cluster every command of
  the protocol has one key, so a session and its lock may live on different primaries; the release script names its one
  key. <a id="SR-6.4"></a> A connection that cannot be made or that fails during a command fails with the kind
  `connection`, and the error tells no address, user or password. A failed request leaves its lock to expire.
  <a id="SR-6.5"></a> A process remembers the process id with which it made its connections; after a `fork` the child
  neither uses nor closes the connections that it inherited, whose sockets the parent still uses, and makes its own.
- **SR-7** <a id="SR-7.1"></a> A PHP process with phpredis shares the sessions when it sets `session.save_handler=redis`,
  `session.serialize_handler=php_serialize`, `redis.session.locking_enabled=1`, `redis.session.lock_expire` to the
  `lock_expire` of the store (never 0, because phpredis then takes `max_execution_time`, which is 0 on the command line),
  `redis.session.lock_retries` to the `lock_retries` of the store (never -1, which tries for ever),
  `redis.session.lock_wait_time` to the `lock_wait` of the store (both in microseconds),
  `redis.session.lock_failure_readonly=0`, `redis.session.early_refresh=0` and `redis.session.compression=none`.
  Another setting is outside the interoperation. <a id="SR-7.2"></a> The `rediscluster` session handler of phpredis
  takes no lock, so on Redis Cluster no PHP process with phpredis shares the sessions.
- **SR-8** <a id="SR-8.1"></a> The waits (plan B-5): `open` and `close` run on the tokio runtime (`async`), and the
  wait between two tries of a lock is a tokio sleep. A caller cancels by dropping the future; a lock that a dropped
  opening took expires after `lock_expire`. <a id="SR-8.2"></a> `lifetime`, `lock_expire`, `lock_retries` and
  `lock_wait` are unsigned integers of the interface contract, which a binding judges as SF-8.2 states; the cases of
  `contracts/redis-sessions.json` that carry `binding` run on the bindings.
- **SR-9** <a id="SR-9.1"></a> The kinds of `polyspec-session-store-redis`:

  | Kind | Status | Category | Rule |
  |---|---|---|---|
  | `invalid_lifetime` | none | `value` | SR-1.2 |
  | `invalid_lock_expire` | none | `value` | SR-1.2 |
  | `invalid_setting` | none | `value` | SR-6.1: an address that is no URL of Redis or an empty list of addresses |
  | `lock_timeout` | 500 | `store` | SR-3.3 |
  | `lock_lost` | 500 | `store` | SR-4.3 |
  | `reply` | 500 | `store` | SR-5.1 |
  | `redis` | 500 | `store` | SR-5.2 |
  | `connection` | 500 | `io` | SR-6.4 |
  | the kinds of SE-9.2 | | | the core |

## Verification

- **SE-10** <a id="SE-10.1"></a> `contracts/sessions.json` holds the cases of the core: group `identifier` (SE-5.1,
  SE-5.2), group `created` (SE-5.3: the `length` and the `alphabet` of a new identifier, which is an identifier and
  differs from the one made before it), group `store` (SE-2, SE-4, run on the memory store and on a record session) and
  group `record` (SE-6, SE-7). `contracts/file-sessions.json` holds the requests of the store of files (SF-1 to SF-5),
  and `contracts/redis-sessions.json` the requests of the store of Redis with the commands that each sends (SR-1 to
  SR-5). <a id="SE-10.2"></a> A PHP value of a case is its serialize form, and a byte string of a case is JSON text
  whose characters U+0000 to U+00FF are its bytes. A failure is the key `raises` with `kind`, `status`, `category` and
  `message`, and such a case has no result key. A case of several requests (`steps`) that fails names in `raises` the
  step and the operation that fail (`step`, `at`: `open`, `call` or `close`); it has no results, and its `records` or
  `stored` hold what the store keeps after the failure. A created identifier of a case is `#n`, the n-th identifier that
  the case creates, counted from 0. The secret of a lock is `<secret>` and the release script `<release>` in a command
  record. A value of a call is also `{"nested": n}`: n arrays that each hold the next at the key 0, the innermost
  holding null, which is the serialize form `a:1:{i:0;` n times, `N;` and `}` n times. A case states in `found`, for
  each step, the identifier that the opening of the step found (SE-7.1), `#n` or null; a case without `found` does not
  compare it. A case of the group `open` of the store of files states the directory of the made store in
  `store_directory`, and one of the store of Redis with `setting` makes the store of that connection setting (one of
  `server`, `sentinel` and `cluster` with its fields, SR-6.1), which connects to nothing. <a id="SE-10.3"></a> Every
  record that a case writes is read again by real PHP (`unserialize` of PHP 8.4) in the tests of the core when a PHP
  binary is given (`PHP`), so the records are those of PHP. <a id="SE-10.4"></a> The commands of the Redis cases run
  against a model of the Redis commands that the protocol sends, and the same requests run against a real Redis server,
  which the tests start when a `redis-server` binary is given (`REDIS_SERVER`). A case with `textReplies` (the model
  gives each record as a status reply, SR-5.1) or with `errorReplies` (the model gives the stated error reply to every
  command of a name, SR-5.2) runs against the model only, because a real server gives neither to these commands on these
  keys.
