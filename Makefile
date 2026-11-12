CARGO := cargo
# The contract check of bindgen (C-0); BINDGEN names a checkout of the bindgen repository at the revision of Cargo.lock.
BINDGEN ?= ../bindgen
# The real peers of the tests, each optional: PHP reads the records back (SE-10.3), redis-server, redis-cli and openssl
# run the stores of Redis against real servers (SE-10.4). A test that needs a missing peer is reported as ignored.
PHP ?= $(shell command -v php)
REDIS_SERVER ?= $(shell command -v redis-server)
REDIS_CLI ?= $(shell command -v redis-cli)
OPENSSL ?= $(shell command -v openssl)
CONTRACTS := packages/core/rust/interface.json packages/file/rust/interface.json packages/redis/rust/interface.json

.PHONY: check test lint-rust fmt-check contract-check

check: lint-rust fmt-check test contract-check

test:
	PHP=$(PHP) REDIS_SERVER=$(REDIS_SERVER) REDIS_CLI=$(REDIS_CLI) OPENSSL=$(OPENSSL) $(CARGO) test --locked --workspace

lint-rust:
	$(CARGO) clippy --locked --workspace --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt --all --check

contract-check:
	for contract in $(CONTRACTS); do node $(BINDGEN)/packages/ts/src/cli.ts check $$contract || exit 1; done
