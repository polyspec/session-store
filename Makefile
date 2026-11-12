CARGO := cargo
# Extra arguments of every cargo build, for example --ignore-rust-version on a machine without the pinned release.
CARGOFLAGS ?=
# The crates of the library; the binding crates of the workspace need make bindings first (bindgen design L-2).
CORE := -p polyspec-sessions -p polyspec-sessions-core -p polyspec-sessions-file -p polyspec-sessions-redis
# bindgen at the commit of Cargo.lock (scripts/bindgen.mts).
BINDGEN := node scripts/bindgen.mts
# The real peers of the tests, each optional: PHP reads the records back (SE-10.3), redis-server, redis-cli and openssl
# run the stores of Redis against real servers (SE-10.4). A test that needs a missing peer is reported as ignored.
PHP ?= $(shell command -v php)
REDIS_SERVER ?= $(shell command -v redis-server)
REDIS_CLI ?= $(shell command -v redis-cli)
OPENSSL ?= $(shell command -v openssl)

.PHONY: check test lint-rust fmt-check bindgen-check bindings

check: lint-rust fmt-check test bindgen-check

test:
	PHP=$(PHP) REDIS_SERVER=$(REDIS_SERVER) REDIS_CLI=$(REDIS_CLI) OPENSSL=$(OPENSSL) $(CARGO) test $(CARGOFLAGS) $(CORE) --locked

lint-rust:
	$(CARGO) clippy $(CARGOFLAGS) $(CORE) --locked --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt $(CORE) --check

# The contract check and the comparison of the tracked binding packages with what bindgen writes (CK-19).
bindgen-check:
	$(BINDGEN) check packages/rust/interface.json

# Writes the generated sources into packages/{php-ext,python,ts}/generated, which are not tracked.
bindings:
	$(BINDGEN) generate --root .
