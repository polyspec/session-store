<!-- doc-id: sessions -->
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
  <a id="SE-9.6"></a> No input reaches some failures of the environment, the kind `internal` and a reply of another
  type than a Redis server gives (SE-10.7), so fault injection points imitate them (binding design G-5, X-1). The module `fault` of `polyspec-session-store-core`, which
  `polyspec-session-store` gives again, names the points as `session_store.<id>`, the crates of the stores call the
  points of their places through it, and the interface contract names the same points in `faults`. A point does nothing
  unless the feature `fault-test` of `polyspec-session-store-core` is on: `polyspec-session-store` turns it on through
  its own feature `fault-test`, which a binding crate turns on for its tests only, the tests of the crates turn it on
  through their dev-dependency `polyspec-binding-core`, and no release build has it. An `io` injection at `open`, `create` or
  `read_dir` is the failure of that call of the system with the `errno` of the fault, whose kind follows SF-9.2, or a
  failure without `errno` of the kind `io` when the fault names none. The `fills` of the contract name `io` alone of
  the kinds of SF-9.2, because the row of the variant `Io` in `errors.kinds` names no `kinds`, and a point fills only
  the kinds of that table; `create` also fills `identifier_taken`, the kind of `EEXIST` at that call (SF-4.4). An `io` injection at `random` is a failure of the random source, which has no `errno`. A
  `broken` injection at `open_reply` or `close_reply` takes the reply of `GET` in the opening or of `EXPIRE` in the
  close of a session that changed nothing as a reply of another type (`Reply::Other`), so the branch of SR-5.1 runs
  with the release that follows it: no state of a Redis server gives such a reply (SE-10.7), while the commands before
  it and the release go to the server, so the case of each point runs against the service of the test run. The
  core starts no task and no thread, so each point takes the arming of the call that reaches it; a binding installs the
  arming of an `open` or a `close` of the store of Redis around each poll of its future (binding GN-6n). Every invariant branch
  of the kind `internal` that `get`, `set` and `remove` of a record session reach, the calls of `file_session` and
  `redis_session`, has a point just before it, which imitates the failed invariant as `broken`; the points of the copy
  are also reached by `close`, which writes the record with the same copy. `get`, `set` and `remove` of `MemoryStore`
  and `random_hex` have no branch of the kind `internal`. Their members `memory_store.get`, `memory_store.set` and
  `memory_store.remove` raise no kind, but their core paths are the methods of the trait `Store`, which return a
  `Result`, so the interface contract states `core.result: true` for them (binding CK-21).

  | Id | Place (function) | Inject | Fills | Rule of the condition | Takes the arming |
  |---|---|---|---|---|---|
  | `random` | `random_hex`, which `new_identifier`, the start of a new session, a renewal and the opening of the store of Redis call | `io` | `random` | SE-5.4 | yes |
  | `internal` | two branches: `renew` of `MemoryStore`, where the count of renewals is treated as passing u64 (SE-4.3), and the start of a record session in `get`, `set`, `remove` and `renew` (`RecordSession::start`), where the state after the start is treated as waiting (SE-7.2); the second is on the path of every caller of `file_session` and `redis_session`, so a contract that passes on the kinds of this library arms the point `internal` for a member of its own (binding X-1) | `broken` | `internal` | SE-9.6 | yes |
  | `record_root` | the decode of a found record at the start of a record session (`Values::decode`): the root is treated as naming no array | `broken` | `internal` | SE-6.3 | yes |
  | `record_index` | `get` of a record session (`Values::get`): the index is treated as naming no entry | `broken` | `internal` | SE-6.5 | yes |
  | `value_reference` | `get` of a record session of a read element that is a reference (`value_of`): the source is treated as naming no reference | `broken` | `internal` | SE-6.5 | yes |
  | `value_root` | `get` of a record session of a read element (`value_of`): the copy is treated as refusing its top value | `broken` | `internal` | SE-6.5 | yes |
  | `copy_new_reference` | the copy of a reference of the source, in `get` and in `close` (`Copier::shallow`): a new reference is treated as refused | `broken` | `internal` | SE-6.6 | yes |
  | `copy_array` | the copy of an array, in `get` and in `close` (`Copier::fill`): the source is treated as naming no array | `broken` | `internal` | SE-6.5 | yes |
  | `copy_element` | the copy of an element of an array, in `get` and in `close` (`Copier::fill`): the element is treated as refused | `broken` | `internal` | SE-6.5 | yes |
  | `copy_object` | the copy of an object, in `get` and in `close` (`Copier::fill`): the source is treated as naming no object | `broken` | `internal` | SE-6.5 | yes |
  | `copy_property` | the copy of a property of an object, in `get` and in `close` (`Copier::fill`): the property is treated as refused | `broken` | `internal` | SE-6.5 | yes |
  | `copy_reference` | the copy of a reference, in `get` and in `close` (`Copier::fill`): the source is treated as naming no reference | `broken` | `internal` | SE-6.6 | yes |
  | `copy_set_reference` | the copy of the value of a reference, in `get` and in `close` (`Copier::fill`): the value is treated as refused | `broken` | `internal` | SE-6.6 | yes |
  | `open` | the open of the session file in `FileSessions::open` | `io` | `io` | SF-3.1, SF-9.2 | yes |
  | `create` | the exclusive creation of the file of a new session in `close` of `file_session` | `io` | `io`, `identifier_taken` | SF-4.4, SF-9.2 | yes |
  | `read_dir` | the read of the directory in `FileSessions::collect` | `io` | `io` | SF-5.1, SF-9.2 | yes |
  | `open_reply` | the reply of `GET` in the opening of a found identifier of the store of Redis (`Opening::next`): the reply is treated as one of another type | `broken` | `reply` | SR-5.1, SE-10.7 | yes |
  | `close_reply` | the reply of `EXPIRE` in the close of a session of the store of Redis that changed nothing (`Closing::next`): the reply is treated as one of another type | `broken` | `reply` | SR-5.1, SE-10.7 | yes |

## The store of files

- **SF-1** <a id="SF-1.1"></a> The store of files keeps each session in a file of one directory, so the server processes
  that share the directory share its sessions. <a id="SF-1.2"></a> The directory is an absolute path of an existing
  directory; another path fails the making of the store with the kind `invalid_directory`, and the error names the
  path. A `session.save_path` of PHP in the form `N;/path` or `N;MODE;/path` (subdirectories by the first characters of
  the identifier) is not an absolute path and fails so: the store keeps every file in its directory.
  <a id="SF-1.3"></a> The lifetime is a positive number of seconds, 1440 when the caller states none, the default of
  `session.gc_maxlifetime` of PHP; 0 fails with the kind `invalid_lifetime`. <a id="SF-1.4"></a> The store of files
  has no WASM binding: a browser has no directory that server processes share, and the store waits on file I/O and on
  a lock of another process, which WebAssembly in a browser cannot do (binding Q-4). The classes `FileSessions` and
  `FileSession` are classes of PHP, Python, Node and Go.
  <a id="SF-1.5"></a> The store is made by the static function `FileSessions::in_directory` of Rust and by the static
  member `file_sessions.in_directory` of the interface contract, which takes the directory and the lifetime and
  returns the store; the class `FileSessions` has no constructor. The making reads the directory (SF-1.2), so it waits
  on file I/O, and a language whose constructor cannot be awaited has no form of a constructor that waits, which
  binding refuses for every target (binding GN-6m).
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
  releases the lock and its turn at once and fails. The cancellation is an `optional` of the type `cancel` of the
  interface contract, whose default the core decides (`{"computed": "SF-3.8"}`): a caller that leaves it out waits
  without one. `FileSessions::open` takes it by value (`Option<Cancel>`), because binding gives the core the
  cancellation that it makes for the call and binds to the value of the language (binding GN-6i, GN-6aq). The
  parameter is for Python and Node (`only`). PHP has none, because a PHP call blocks its thread and the store of files
  takes no cancellation from another process or from a callback while `open` waits, so nothing could fire it during
  the call (plan B-5). Go has none, because binding takes a cancellation of Go only as the first parameter `ctx`
  (GN-6aq) and the parameter follows the required `cookie`.
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
  value that is no integer of 0 or more before the store runs: a negative or fractional number breaks the rule of the
  number and is refused with the kind `invalid_number`, and a text or a boolean is no number and is refused with the
  kind `invalid_argument_type` (binding generator GN-12a, B2.412). Such cases
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

  The options are the record `store_options` of the interface contract, whose Rust type is `StoreOptions`
  (`core_type`), each field `None` for its default; binding names the Rust type of a record that a parameter takes by
  its `core_type`.

  <a id="SR-1.3"></a> Every process of one Redis server gives the options the same values, and `lock_expire` is longer
  than any request. `lock_expire` has no default, because no value is longer than every request of every application
  and phpredis has none either (its 0 takes `max_execution_time`); the caller states it.
  <a id="SR-1.4"></a> The store of Redis has no WASM binding: a browser opens no TCP connection to a Redis server, and
  the client of the store runs on the sockets of Tokio, which do not build for WebAssembly. The classes `RedisSessions`
  and `RedisSession` are classes of PHP, Python, Node and Go, and the library built for wasm32 holds neither the store
  of files nor the store of Redis.
  <a id="SR-1.5"></a> The library built for wasm32 takes the random bytes of a new identifier (SE-5.3) from
  `crypto.getRandomValues` of its host through the backend `wasm_js` of `getrandom` 0.3: the crate of the core enables
  the feature `wasm_js` for wasm32 only, and `.cargo/config.toml` sets `--cfg getrandom_backend="wasm_js"` for the
  target `wasm32-unknown-unknown`. An identifier needs a cryptographic random source, and both a browser and Node give
  one through that function; without the backend `getrandom` 0.3 does not build for wasm32.
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
  an ACL user and password. <a id="SR-6.1b"></a> A store that connects with TLS (`rediss://`, or Sentinel with `tls`)
  reads the trust store of the machine once in a process and gives its authorities to every client of TLS that it
  builds: of a server, of the Sentinel nodes and of the primary that they name, and of the nodes of a Cluster. A trust store that cannot be read fails the connection with the kind
  `connection` (SR-6.4) and is read again by the next connection. The Sentinel addresses are all `rediss://` or all
  `redis://` and share one user and password; another list fails with the kind `invalid_setting`. <a id="SR-6.2"></a> With Sentinel, a request that sends a command asks Sentinel for the
  primary of its service before its first command, also when a connection to the former primary is open, because a
  former primary accepts writes for some seconds after a promotion. <a id="SR-6.3"></a> With Cluster every command of
  the protocol has one key, so a session and its lock may live on different primaries; the release script names its one
  key. <a id="SR-6.4"></a> A connection that cannot be made or that fails during a command fails with the kind
  `connection`, and the error tells no address, user or password. A failed request leaves its lock to expire.
  <a id="SR-6.5"></a> A process remembers the process id with which it made its connections; after a `fork` the child
  neither uses nor closes the connections that it inherited, whose sockets the parent still uses, and makes its own.
  <a id="SR-6.6"></a> The TLS and the
  user and password of a server whose address the setting gives are those of its URL: the scheme `rediss://` and the
  user and password of the URL, for the URL of a server and for the addresses of a Cluster. The addresses of a Cluster
  are all `rediss://` or all `redis://` and share one user and password; another list fails with the kind
  `invalid_setting`. The primary of Sentinel has no address in the setting, so `tls`, `user` and `password` of the
  Sentinel setting are those of the primary. <a id="SR-6.7"></a> Each setting takes `authorities`, the path of a file
  of PEM certificates, or none. With it every connection of TLS of the setting, to a server, to the Sentinel nodes and
  their primary and to the nodes of a Cluster, trusts the authorities of that file alone, which the store reads at the
  first connection of TLS and once in a process for a path; without it the connections trust those of the trust store
  of the machine (SR-6.1b). A setting that gives `authorities` and connects nowhere with TLS, and a file that cannot be
  read or that holds no PEM certificate, fail with the kind `invalid_setting`, whose text names the path.
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
  | `invalid_setting` | none | `value` | SR-6.1, SR-6.6, SR-6.7: an address that is no URL of Redis, an empty list of addresses, addresses of Sentinel or of a Cluster that differ in their scheme, user or password, `authorities` in a setting without TLS, or a file of `authorities` that cannot be read or holds no PEM certificate |
  | `lock_timeout` | 500 | `store` | SR-3.3 |
  | `lock_lost` | 500 | `store` | SR-4.3 |
  | `reply` | 500 | `store` | SR-5.1 |
  | `redis` | 500 | `store` | SR-5.2 |
  | `connection` | 500 | `io` | SR-6.4 |
  | the kinds of SE-9.2 | | | the core |

## Verification

- **SE-10** <a id="SE-10.1"></a> `contracts/sessions.json` holds the cases of the core: group `identifier` (SE-5.1,
  SE-5.2), group `created` (SE-5.3: the `length` and the `alphabet` of a new identifier, which is an identifier and
  differs from the one made before it, and SE-5.4), the groups `memory_store.<member>.made`
  (SE-2, SE-4), which the interface contract declares: a call of each member of the memory store on a store that
  `memory_store.new` and the calls of `then` made, whose result and the calls of `after`, in their order and with
  `renewals` read as a property, show the store after it; a case that states `record`, the record that the same
  calls write on a record session, is run on both stores by the Rust tests, which alone read `record`, because no
  member gives the record (SE-1.2), group `store` (SE-2, SE-4, run on the memory store and on a record session), which
  holds the case whose value shares an object and a reference: its record numbers them as the record session writes
  them, which only the Rust tests compare
  (binding GN-13f), group `record` (SE-6, SE-7), group `internal` (SE-9.6) and group `recordFaults` (SE-9.6). `contracts/file-sessions.json` holds the requests of the store of files (SF-1 to SF-5),
  and `contracts/redis-sessions.json` the requests of the store of Redis with the commands that each sends (SR-1 to
  SR-5). <a id="SE-10.2"></a> A PHP value of a case is its serialize form, and a byte string of a case is JSON text
  whose characters U+0000 to U+00FF are its bytes. A case names the rule statements that it checks in `rule`, a
  list, and states what it shows in `note`, the keys of a case that the generator reads as no input (binding CK-96). A failure is the key `raises` with `kind`, `status`, `category` and
  `message`, and such a case has no result key. A case of several requests (`steps`) that fails names in `raises` the
  step and the operation that fail (`step`, `at`: `open`, `call` or `close`); it has no results, and its `records` or
  `stored` hold what the store keeps after the failure. A created identifier of a case is `#n`, the n-th identifier that
  the case creates, counted from 0. The secret of a lock is `<secret>` and the release script `<release>` in a command
  record. A value of a call is also `{"nested": n}`: n arrays that each hold the next at the key 0, the innermost
  holding null, which is the serialize form `a:1:{i:0;` n times, `N;` and `}` n times. Those two forms are of the
  group `requests`; a case of one call, which a generated binding test runs, gives a PHP value as a value of the alias
  `php_value` (`{"kind": "int", "value": 1}`, and `{"kind": "list", "repeat": n, "value": <value>}` for n nested
  lists, binding G-5), which the Rust tests read in the same form. A case states in `found`, for
  each step, the identifier that the opening of the step found (SE-7.1), `#n` or null; a case without `found` does not
  compare it. A path of a case of one call (`directory`, `store_directory`) is a relative path in a directory that the
  test makes for the case, `.` for that directory itself, as a generated binding test resolves a case value of the
  type `path` (binding GN-6y), and the Rust tests resolve it the same way. A case of one call that needs a stored file
  gives it in `files`, an object from the relative path of the file to its bytes, a text for its UTF-8 or
  `{"hex": ...}`, which the test writes with the mode 0600 into that directory before it makes any value of the case
  (binding GN-13i); the group `requests` gives its files as a list of `identifier` (or `name`), `bytes` and `age`
  instead, which the Rust tests alone read. A directory that is not absolute (SF-1.2: a relative path and a
  `session.save_path` with levels) has no case value of the type `path`, so those cases are the group
  `openNotAbsolute`, which the interface contract does not declare and the Rust tests run. The group `open` of the store
  of files is read by `file_sessions.in_directory`, and a case of it that makes a store states its directory in
  `store_directory` and its lifetime in `store_lifetime`; the groups `file_sessions.lifetime` and
  `file_sessions.directory` hold the cases that make a store, with the result of that member in `store_lifetime` and
  `store_directory`, because a failure of the making is raised by `in_directory` and not by a property (binding
  CK-22, CK-96); and every case of the group `open` of the store of Redis gives `setting` and `options`, the two
  parameters of `redis_sessions.new`, and makes the store of that connection setting, which connects to nothing. A
  setting of a case, at any place, is a value of the variant `setting`: its tag `kind` names `server`, `sentinel` or
  `cluster`, and the fields of that case are beside it (`{"kind": "server", "url": "..."}`, SR-6.1, binding G-5); a case that tests the options alone names the server
  of a socket under `/dev/null`. A generated binding test takes the keys of a case as the arguments of one call and
  takes no case that leaves out a required field (binding G-5), so the case whose options give no `lock_expire` is the
  group `openMissing`, which the interface contract does not declare and the case runners of the bindings run. <a id="SE-10.3"></a> Every
  record that a case writes is read again by real PHP (`unserialize` of PHP 8.4) in the tests of the core, which fail
  when no PHP binary is given (`PHP`), so the records are those of PHP. <a id="SE-10.4"></a> The commands of the Redis cases run
  against a model of the Redis commands that the protocol sends, and the same requests run against a real Redis server.
  Every real server of the tests (the server of the cases, and the ACL user, TLS, Sentinel and Cluster of SR-6) is
  `redis-server` of Redis 8.10.3 in a container from the image that the Makefile pins by its digest (`REDIS_IMAGE`,
  pulled by `make redis-image`), and no test runs a server that is installed on the machine. bay starts and removes
  the servers: it runs a Compose project on Apple `container` on Darwin and on Docker on Linux, so the image and its
  digest are one, the tests of both platforms connect to the same bytes of the server, and no file of this repository
  holds a command of an engine. `scripts/servers.sh` holds the commands of bay (`check`, `pull`, `plan`, `up`,
  `images`, `exec`, `logs`, `output`, `down`); the harness and the run start that script alone.
  - A project for each test. A test writes the Compose file of the servers that it needs into a directory of its own
    below the directory of the run (`SESSION_STORE_PROJECTS`) and starts it as one project by one `up`: one service
    for each server, the image by its digest, the arguments of the server as `command`, the port of the server as
    `expose`, a directory of the project as a volume where the server reads a file of the test, and no published
    port. The arguments of a server are part of what a test states (an ACL user, TLS, Sentinel, the cluster mode), and
    the keys, the users and the cluster of one test must not reach another test, so no server is shared between tests
    and a run starts only the servers of the tests that it runs. The cases of `tests/contract.rs` are one test in
    this sense: they share one server, each case in a database of its own.
  - The name of a project is the name of its directory, which bay takes when the file states none: the name of the
    directory of the run, the id of the test process and a count. The directory of the run is named after the
    process of the run, so no two runs of a machine hold one name. bay refuses a project whose name holds another
    character than a letter, a digit, `-` or `_`, so every directory of a project that the harness or
    `scripts/servers.sh` makes is named from those characters alone; the name that `mktemp -d` gives without a
    template holds a dot.
  - bay gives every name. Before the start the harness reads `bay plan --json` of the Compose file: the host name of
    each service and the paths of the certificate that bay issues for that name. A server that names another one
    takes the name from the plan, as the configuration of Sentinel names its primary, so one `up` starts every server
    of a test. After the start the harness reads the output of `bay up --json` (`address`, the name and the port,
    and `ipv4`), and a test connects to that address. No file of this repository states the domain of the names or a
    path of a certificate, and no port is published to the host. The harness fails a plan or an output of `up` whose
    `schemaVersion` is not 1, the version of the contract that it reads.
  - The server of TLS takes the certificate that bay issues for its name, at the paths in its container that the
    plan gives. No test makes a certificate. The store verifies a server against the trust store of its process,
    because a connection setting names no authority (SR-6.1), and the test of TLS trusts the authority of bay through
    the trust store of the machine, to which `bay install` adds it (the keychain on macOS, the certificate store of
    the system on Linux) and whose entry `bay check -scope run` checks at the start of a run. No test reads the file
    of the authority that the plan names: bay keeps it below the home of the user who runs bay, which is root on the
    runner of CI, and the test process does not run as root, so a client that is given that file fails to read it
    before it connects. No test changes the environment of its process either, because the environment belongs to all
    threads of the process and a thread that changes it while another reads it is not safe; the test of TLS is a
    trial of `tests/server.rs`.
  - The return of `up` is the event of a start: bay returns after every service accepts connections on its port. The
    harness holds no time limit, no sleep and no repeated question. The test of the cluster creates the cluster with
    `redis-cli --cluster create` of the image, which `bay exec` runs in the container of the first node with the
    IPv4 addresses of the three nodes. That command is how Redis states the making of a cluster, and it is no service
    of the project: it takes the addresses of the nodes, which bay gives only after the start (a name of a service
    gives more than one address, and a node tells its clients the address that the command gave), and `bay exec`
    returns its status to the test. The fact that the cluster serves is the line in which a node reports the state
    ok, which each node writes once when its state changes: the test waits for that line of each node in the output
    that `bay logs -f` gives from its first line. It does not read `CLUSTER INFO`, which states the state of one
    moment, so that a read before the change would have to be repeated.
  - No test pulls an image: the harness starts a project with `up -no-pull`, which fails before it creates anything
    when the image is not installed, and a test that fails this way names `make redis-image`; no such test is
    reported as ignored. `make redis-image` pulls with `bay pull`, the only step that pulls: `scripts/servers.sh pull`
    writes the image as the one service of a project in a directory `session-store-pull-<letters>` of its own below
    `TMPDIR`, pulls it and removes the directory.
  - bay is a tool that the machine gives, as make is: a run starts `bay` of PATH and neither pins nor compares its
    version. What changes a result of the harness is not the number of a release of bay but the contract of the
    output that the harness reads, so the harness fails a plan or an output of `up` whose `schemaVersion` is not 1,
    and the bay of a development machine, which is often a new build, runs the tests as it is. The run writes the
    output of `bay version` as its first line, the fact of what started its servers. A run fails at its start, before
    the tests are built and before anything is created, when bay is not found or gives no version, with what
    installs it, and when `bay check -scope run` reports that a part of the machine setup that a project needs is
    missing, with `bay install`, which applies it. No run changes the setup of the machine, and `up` changes none
    either; the check at the start gives one failure before the build instead of one in every test after it.
  - A release of bay is pinned for CI alone. The runner of CI holds no bay, so the workflow must download one, and a
    download is used only when a tracked file names it: `config/toolchain.json` names the release that CI installs,
    with the sha256 of its archive of each platform. `make install-bay` downloads the archive of that release for the
    platform, fails when its sha256 is not the pinned one and puts `bay` and `bayd` into `var/tools/bay`. No path of
    a run reads the pin.
  - How bay is started is one value, `BAY` of the Makefile, which `scripts/servers.sh` alone reads; no other file
    starts bay. Its default is `bay` of PATH. The setup of bay is state of the machine (the resolver, the DNS agent,
    the authority) that `bay install` writes for the user who runs it, so on a development machine bay is the one
    that its owner installed and set up. The setup is applied once per machine and never by the recipe of a job, and
    no command of bay runs with `sudo`, because bay keeps its state below the home of the user who runs it, so the
    setup, the check, the pull and every command of a test run as one user. On a runner of CI the action
    `.github/actions/bay` runs before the run step of each job that runs bay: `make install-bay`, `make bay-path`,
    which puts `var/tools/bay` on the PATH of the later steps, and `make setup-bay`, which runs `bay install`, the one
    target that changes the setup of a machine and that a developer runs by choice. The recipes `ci-test` and
    `ci-bindings` then check the setup with `make redis-image` and fail naming `bay install` where it is missing.
  - `make test` runs the tests through `scripts/test-run.mts`, which the node of the checkout runs and which owns the
    directory of the run. A Compose file is written before its `up`, so every project that started has its file.
    The run starts the command of the tests as the leader of a process group of its own, in a session of its own
    (`spawn` with `detached`, which calls `setsid`), passes SIGINT and SIGTERM that it receives to that group, and
    waits for the exit of the command, which is the event that ends the wait. A shell script cannot do this: a POSIX
    shell gives a script no way to make a process group, `set -m` needs a terminal in dash, the `/bin/sh` of Debian
    and Ubuntu, where the command then stays in the group of the run and ignores SIGINT, and macOS has no `setsid`
    program. When the run ends, fails or is stopped by SIGINT or SIGTERM, the script waits for the tests, runs `down` for every Compose file that the directory still
    holds and removes the directory. An `up` that the signal stops removes what it created. A project that `down`
    does not remove fails the run, which then keeps the directory and names the file. A run and its command that
    SIGKILL ends remove nothing, so a run writes its mark, the id and the start of its process, into its directory,
    and a run that starts removes the run whose mark names a process that is gone: no process has that id, or the
    process that has it started at another time, because a process id is used again. It runs `down` for every Compose
    file of that directory, removes the directory and writes which run and how many projects it removed. A run whose
    process lives and a directory without a mark are left as they are.

  A case with `text_replies` (the model gives each record as a status reply, SR-5.1), with `status_replies` (the model
  gives the status reply `OK` to every command of a name that it lists, SR-5.1) or with `error_replies` (the model gives
  the stated error reply to every command of a name, SR-5.2) runs against the model only and has no trial against a
  server, because a real server gives none of them to these commands on these keys.
  <a id="SE-10.5"></a> A case with `faults` arms the fault injection points of SE-9.6 for its one call, as a generated
  binding test does (binding design G-5): the Rust tests arm the list for the member, run the call with the arming that
  the entry point of the member takes, and fail the case when a fault did not fire. Such a case is a case of one call in
  a group that only its member reads, so that the test of no other member arms it: the groups `created`
  (`new_identifier`) and `internal` (`memory_store.renew`) of `contracts/sessions.json`, the group `recordFaults` of
  `contracts/sessions.json`, a call of a record session for each point of a record session, which only the Rust tests
  read and which runs the same call without the arming first, so that the input alone gives no failure, the groups
  `openFaults`
  (`file_sessions.open`) and `collectFaults` (`file_sessions.collect`) of `contracts/file-sessions.json`, and the group
  `openFaults` (`redis_sessions.open`) of `contracts/redis-sessions.json`. A case of one call makes its receiver with
  the constructor of its unit, or with `file_sessions.in_directory` for the store of files (SF-1.5), from the keys of
  the case (binding T-5): the store of files of the directory of the test (`.`), whose read the armed point fails
  before the call reads it, and the store
  of Redis of a socket under `/dev/null`, which no file can be. The group `opening` of `contracts/redis-sessions.json`
  holds the input case of one call whose connection fails with that setting (SR-6.4); no injection imitates a failure
  that this input reaches. <a id="SE-10.6"></a> A session of a request (`file_session`, `redis_session`) is made by
  `open` of its store, a method of another unit, so a case of one of its calls gives its receiver under the key
  `receiver` as an instance value that earlier calls make (binding GN-13f): `open` without a cookie of a store made by
  its constructor or by `file_sessions.in_directory` (SF-1.5), the store of files of the directory of the test (`.`)
  with a cancellation that does not fire and the store of
  Redis of a socket under `/dev/null`, and the calls of `then`. The groups `file_session.<member>.made` and
  `redis_session.<member>.made` of `get`, `set`, `remove` and `renew` arm `session_store.random`, which the start of the
  new session reaches (SE-5.4, SE-7.2); `redis_session.close.made` closes a session that set a value, whose close
  connects to that socket and fails with `connection` (SR-6.4). `file_sessions.open.cancel` gives `cancelled` of
  `file_sessions.open` (SF-3.8) by a cancellation that fired before the call (`{"cancel": "fired"}`).
  Its case `an_opening_without_a_cancellation_opens` leaves the cancellation out, whose default the core decides, and
  opens the session of a found identifier without a wait (SF-3.8, binding CK-103).
  `file_session.close.made` gives `io` and `identifier_taken` of `file_session.close`: the point
  `session_store.create` fails the creation of the file of a new session with `EIO` and with `EEXIST` (SF-4.4, SF-9.2),
  because no input reaches either kind on every platform of the library. The cases of one call also reach what a
  session made without a cookie gives: `file_session.close.made` gives `record_write` of `file_session.close` by a set
  value of 4096 nested lists, which the record nests one level deeper than php-serialization writes (SE-6.8), and the
  groups `file_session.found.made` and `redis_session.found.made` give `found` of a session that found no identifier
  (SE-7.1). The groups `file_session.<member>.stored` of `get`, `set`, `remove` and `renew` open, with the cookie of
  64 `e`, the session whose file `sess_<id>` the case writes (`files`): a record that holds no array gives
  `record_not_array` (SE-6.4), and bytes that unserialize refuses give `record` (SE-6.3).
  The group `requests` holds cases of several requests (`steps`), which the generator of binding refuses by name
  (binding G-5), so the interface contract does not declare it, and the Rust tests alone run it: `tests/contract.rs` of
  the store of files, and that of the store of Redis against the model and a Redis server (SE-10.4). No case of the
  group `requests` arms a point. <a id="SE-10.7"></a> A kind of the store of Redis that only a state of a Redis server
  reaches, a stored record, a lock that another holder took or a key of another type, has a case of one call that runs
  against the service `redis` of the test run (binding GN-13j, CK-95), because a stored record and a reply of a server
  are inputs, which no fault injection point imitates (SE-9.6); a reply of another type, which no state reaches, is
  imitated by a point whose case runs against the same service. The interface contract declares the service in
  `services`: its `address` is the variable `SESSION_STORE_REDIS_URL`, the URL `redis://<name>:<port>` of the server,
  and its `state` is the variable `SESSION_STORE_REDIS_STATE`, the absolute path of `scripts/redis-state.sh`, which runs
  `scripts/redis-state.mts` on the node of the checkout.
  - The run. `scripts/test-run.mts --service redis` starts the server for the generated binding tests of `make
    test-python`, `make test-node`, `make test-php` and `make test-go`, each target in a run of its own: it writes the
    Compose file of one project in the directory of the projects of the run with the one service `redis`,
    `redis-server` of `REDIS_IMAGE` with the arguments of the server of the cases (SE-10.4), starts it by one `up`,
    reads the address of the service from the output of `up`, sets both variables for the tests, and removes the
    project with the other projects of the run when the run ends, fails or is stopped. A start that fails, an image
    that is not installed (`make redis-image` pulls it) and an output of `up` of another `schemaVersion` or without the
    service fail the run before the tests. The WASM tests run without the service: the core of WASM has no socket, so
    each row of `cases` of a group of such cases has `only` for `php`, `python`, `node` and `go`, with this rule as its
    reason (binding GN-13h, CK-95). The job `bindings` of the workflow `ci` uses the action bay and runs `make ci-bindings`,
    whose recipe pulls the image as `make ci-test` does before the tests.
  - The program. The test runs it as `<program> setup|change|check <scope>` with a JSON object on its standard input,
    which maps the name of a key after the scope to its value: a text, whose characters U+0000 to U+00FF are the bytes
    of a string (SE-10.2), or `{"list": [<text>, ...]}`, a list of at least one such item; in `change`, null removes
    the key. `setup` removes every key whose name starts with the scope and writes each key of the object, so that the
    scope holds exactly that state; `change` writes each key of the object in place of what the key held, also a key of
    another type, and removes each key that is null, as another client of the server does while the session lives;
    `check` reads every key of the scope and exits with 0 when they are exactly the keys of the object with those
    values, and otherwise prints each key that differs with its expected and its found value and exits with 1. The
    program speaks the protocol of Redis (RESP2) itself over a socket of `node:net`, so it needs no package; it sends one
    command and reads its reply before the next and waits for no time. An error reply, a connection that fails or
    closes, and an input of another form fail it with the status 1 and the reason. `make redis-state-test` runs it
    against a stand-in of a server that the test serves.
  - The scope. A case gives its store the scope, `<target>:<member id>:<case id>:`, as the option `prefix` (SR-1.2)
    by the value `{"service": "scope"}`, and the URL by `{"service": "address"}`, so the key of its session and of its
    lock (SR-2.1) are keys of its scope, and no two cases, of one target or of two targets that run at once, share a key
    on one server.
  - The groups. `redis_sessions.open.served`, `redis_session.<member>.served` of `get`, `set`, `remove` and `renew` and
    `redis_session.close.served` of `contracts/redis-sessions.json` hold the cases, whose session has the identifier of
    32 `a`. `lock_timeout` of `redis_sessions.open`: the setup stores the session and the secret `another holder` in
    its lock key, and the store tries the lock once (`lock_retries` 0); the check finds both keys unchanged. `redis` of
    `redis_sessions.open`: the key of the session holds a list, so `GET` gets the error reply `WRONGTYPE` after the
    lock is taken, and the release follows. `record_not_array` and `record` of `get`, `set`, `remove` and `renew`: the
    stored key holds `i:1;` (SE-6.4) and bytes that unserialize refuses (SE-6.3). `lock_lost` of `redis_session.close`:
    a call of `then` sets a value, and its `change` writes the secret of another holder into the lock key, so the close
    finds another secret and writes nothing (SR-4.3). `redis` of `redis_session.close`: the `change` makes the lock key
    a list, so the `GET` of the lock gets the error reply `WRONGTYPE`; the release script then fails on the same key,
    and its error does not replace the first one (SR-4.5). `record_write` of `redis_session.close`: a new session sets
    a value of 4096 nested lists, and its close connects to the server and fails before it writes a key. The Rust tests
    run of these groups only the cases that arm a point (below), which need no server; the other cases need the
    program of the run, and the trials of the table below reach the same kinds.
  - No state of a Redis server reaches `reply` of `redis_sessions.open` and of `redis_session.close`: a Redis server
    answers `GET` with a bulk string, nil or an error reply, `EXISTS`, `EXPIRE`, `DEL` and the release script with an
    integer or an error reply, `SET ... NX PX` with `OK`, nil or an error reply and `SETEX` with `OK` or an error reply,
    whatever its keys hold; only a server that does not follow the protocol of Redis gives a reply of another type. So
    each is a failure that no input reaches, which a fault injection point imitates (SE-9.6): `open_reply` and
    `close_reply`. The case of each runs against the service and arms its point for its one call: the opening of a
    stored session whose `GET` is armed (`a_reply_of_another_type_to_the_read_fails_the_opening_with_reply`) and the
    close of a stored session that only read a value, whose `EXPIRE` is armed
    (`a_reply_of_another_type_to_the_expiry_fails_the_close_with_reply`); each releases its lock, and its check finds
    the session unchanged and no lock key. The Rust tests run these two cases against the model, with the scope
    `rust:<member id>:<case id>:` and the keys of the setup, and arm the same point (`model_served::`).

  Each kind and the trials of `packages/redis/rust/tests/contract.rs` that reach it (`model::` against the model,
  `server::` against the server), with the case of one call of the service:

  | Member | Kind | Trials | Case of the service |
  |---|---|---|---|
  | `redis_sessions.open` | `lock_timeout` | `model::` and `server::a_request_that_finds_the_lock_taken_fails_and_changes_nothing` | `a_lock_that_another_holds_fails_the_opening_with_lock_timeout` |
  | `redis_sessions.open` | `reply` | `model::a_reply_that_is_not_bytes_fails_and_releases_the_lock`, `model_served::a_reply_of_another_type_to_the_read_fails_the_opening_with_reply` | `a_reply_of_another_type_to_the_read_fails_the_opening_with_reply`, which arms `open_reply` |
  | `redis_sessions.open` | `redis` | `model::an_error_reply_to_the_read_fails_and_releases_the_lock` | `a_session_key_of_another_type_fails_the_opening_with_redis` |
  | `redis_session.get` | `record_not_array`, `record` | `model::` and `server::a_key_that_holds_no_array_fails_the_call`, `a_key_that_cannot_be_decoded_fails_the_call` | `a_stored_key_that_holds_no_array_fails_get_with_record_not_array`, `a_stored_key_that_cannot_be_decoded_fails_get_with_record` |
  | `redis_session.set` | `record_not_array`, `record` | `model::` and `server::a_key_that_holds_no_array_fails_set`, `a_key_that_cannot_be_decoded_fails_set` | the same for `set` |
  | `redis_session.remove` | `record_not_array`, `record` | `model::` and `server::a_key_that_holds_no_array_fails_remove`, `a_key_that_cannot_be_decoded_fails_remove` | the same for `remove` |
  | `redis_session.renew` | `record_not_array`, `record` | `model::` and `server::a_key_that_holds_no_array_fails_renew`, `a_key_that_cannot_be_decoded_fails_renew` | the same for `renew` |
  | `redis_session.close` | `lock_lost` | `model::` and `server::a_request_that_lost_its_lock_writes_nothing_and_fails` | `a_lock_that_another_took_fails_the_close_with_lock_lost` |
  | `redis_session.close` | `reply` | `model::a_reply_of_another_type_to_the_expiry_fails_the_close_and_releases_the_lock`, `model_served::a_reply_of_another_type_to_the_expiry_fails_the_close_with_reply` | `a_reply_of_another_type_to_the_expiry_fails_the_close_with_reply`, which arms `close_reply` |
  | `redis_session.close` | `redis` | `model::an_error_reply_to_the_write_fails_and_releases_the_lock` | `a_lock_key_of_another_type_fails_the_close_with_redis` |
  | `redis_session.close` | `record_write` | `model::` and `server::a_value_too_deep_for_a_record_fails_the_close` | `a_value_too_deep_for_a_record_fails_the_close_with_record_write` |

## PHP release isolation

PHP assets require a fresh job for every platform and supported PHP minor, with one setup before one build and a unique platform-and-minor artifact. The non-PHP native assets build once per platform. Explicit include-only matrices define exactly these jobs; extra axes, exclusions and repeated native owners are refused. Each PHP build passes `EXPECTED_PHP_MINOR` from its own setup input, because a process can retain a loaded library after PATH changes. Workflow owner verification and native asset verification are separate: the latter remains pending until matching released recipes are installed.
