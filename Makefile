CARGO := cargo
# Extra arguments of every cargo build, for example --ignore-rust-version on a machine without the pinned release.
CARGOFLAGS ?=
# The crates of the library; the binding crates of the workspace need make bindings first (binding design L-2).
CORE := -p polyspec-session-store -p polyspec-session-store-core -p polyspec-session-store-file -p polyspec-session-store-redis
# binding at the commit of Cargo.lock (scripts/binding.mts).
BINDING := node scripts/binding.mts
# The real peers of the tests, each optional: PHP reads the records back (SE-10.3), redis-server, redis-cli and openssl
# run the stores of Redis against real servers (SE-10.4). A test that needs a missing peer is reported as ignored.
PHP ?= $(shell command -v php)
REDIS_SERVER ?= $(shell command -v redis-server)
REDIS_CLI ?= $(shell command -v redis-cli)
OPENSSL ?= $(shell command -v openssl)

.PHONY: check test lint-rust fmt-check binding-check bindings

check: lint-rust fmt-check test binding-check

# A peer that is not found is not named to the tests: a variable that is set and empty would name a binary without a path.
PEERS := $(foreach peer,PHP REDIS_SERVER REDIS_CLI OPENSSL,$(if $($(peer)),$(peer)=$($(peer))))

test:
	env $(foreach peer,PHP REDIS_SERVER REDIS_CLI OPENSSL,-u $(peer)) $(PEERS) $(CARGO) test $(CARGOFLAGS) $(CORE) --locked

lint-rust:
	$(CARGO) clippy $(CARGOFLAGS) $(CORE) --locked --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt $(CORE) --check

# The contract check and the comparison of the tracked binding packages with what binding writes (CK-19).
binding-check:
	$(BINDING) check packages/rust/interface.json

# Writes the generated sources into packages/{php-ext,python,ts}/generated, which are not tracked.
bindings:
	$(BINDING) generate --root .
