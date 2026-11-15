# The shared tools of polyspec, which make bootstrap installs from package-lock.json into node_modules: each recipe runs
# a utility of the installed @polyspec/kit with explicit arguments.
KIT := $(CURDIR)/node_modules/@polyspec/kit/dist
# The tools of every run are the releases that the tracked files name: npm in devEngines.packageManager of package.json,
# Node.js and npm in mise.toml at the digests of mise.lock, mise in config/toolchain.json and Rust in
# rust-toolchain.toml. make install-tools installs them into var/tools of the checkout, never into the machine; the
# shims of mise find its installs and mise.toml through the MISE_* folders below. A recipe starts a tool by its absolute
# path: make runs a recipe line without shell syntax directly, so a bare name would be looked up on the PATH that make
# read at its start.
VAR_TOOLS := $(CURDIR)/var/tools
NODE := $(VAR_TOOLS)/bin/node
NPM := $(VAR_TOOLS)/bin/npm
MISE := $(VAR_TOOLS)/bin/mise
export MISE_DATA_DIR := $(VAR_TOOLS)/mise
export MISE_CACHE_DIR := $(VAR_TOOLS)/mise/cache
export MISE_STATE_DIR := $(VAR_TOOLS)/mise/state
export MISE_CONFIG_DIR := $(VAR_TOOLS)/mise/config
export MISE_GLOBAL_CONFIG_FILE := $(CURDIR)/mise.toml
export MISE_GLOBAL_CONFIG_ROOT := $(CURDIR)
export PATH := $(VAR_TOOLS)/bin:$(VAR_TOOLS)/mise/shims:$(PATH)
CARGO := cargo
# Extra arguments of every cargo build, for example --ignore-rust-version on a machine without the pinned release.
CARGOFLAGS ?=
# The crates of the library; the binding crates of the workspace need make bindings first (binding design L-2).
CORE := -p polyspec-session-store -p polyspec-session-store-core -p polyspec-session-store-file -p polyspec-session-store-redis
# The library crate, whose name the binding crates take with the suffix of their target.
LIBRARY := polyspec-session-store
# The five targets of the contract (binding BD-27).
TARGETS := php python node go wasm
# binding, the npm package @polyspec/binding of the release that package.json pins by the URL of its asset and
# package-lock.json by its integrity; make install-packages installs it with the other packages. Its release and the tag
# of polyspec-binding-core in the manifests are one release (binding CK-20).
BINDING := $(NODE) $(CURDIR)/node_modules/@polyspec/binding/dist/cli.js
# The real peers of the tests, each required: PHP reads the records back (SE-10.3), and bay runs the Redis servers of the
# stores of Redis from the image REDIS_IMAGE as Compose projects, on Apple container on Darwin and on Docker on Linux,
# whose commands scripts/servers.sh holds, and issues their certificates of TLS (SE-10.4). A test whose peer is not
# given fails and names what gives it.
PHP ?= $(shell command -v php)
# How bay is started, the one value that scripts/servers.sh reads (SE-10.4): bay of PATH, a tool that the machine
# gives as it gives make, whose version no recipe compares, with its machine setup, which `bay install` applies once per
# machine. No recipe of a check applies that setup; the run checks it with `bay check` and fails naming `bay install`.
# On a runner of CI, the action .github/actions/bay gives both before the run step of the job.
BAY ?= bay
# Redis 8.10.3, the highest stable release of Redis, in the official image of Debian trixie, pinned by the digest of its
# index; make redis-image pulls it, and the tests fail while it is not installed, so that no test pulls an image.
REDIS_IMAGE := docker.io/library/redis:8.10.3-trixie@sha256:de1068af6f552240dfab67bcc9bfef99bfc60f776bb4580ef077ea4c8c97e848
VAR := $(CURDIR)/var
export CARGO_TARGET_DIR ?= $(CURDIR)/target
# The runtimes and tools of the bindings. PHP is also the binary whose headers the extension is built against
# (PHP_CONFIG) and that loads it; both are paths, because ext-php-rs reads PHP as the path of a file. TSC, the
# TypeScript of package.json, checks the generated declarations; PHPUNIT, the PHPUnit of composer.lock that make
# install-php-tools installs into vendor, runs the PHP tests; MATURIN, the maturin of mise.toml, builds the wheel of the
# tests. PYTHON is found on PATH, where the shims of mise come first, so it is the Python of mise.toml.
PYTHON ?= python3.14
PHP_CONFIG ?= $(shell command -v php-config)
PHPUNIT ?= $(CURDIR)/vendor/bin/phpunit
TSC ?= $(CURDIR)/node_modules/.bin/tsc
MATURIN ?= $(VAR_TOOLS)/mise/shims/maturin
# The Composer of mise.toml, with its home and cache under var/tools, which installs the PHPUnit of composer.lock.
COMPOSER := $(VAR_TOOLS)/mise/shims/composer
export COMPOSER_HOME := $(VAR_TOOLS)/composer
export COMPOSER_CACHE_DIR := $(VAR_TOOLS)/composer/cache
# The Go of the go directive of packages/go/go.mod and the wasm-bindgen CLI of the release of the wasm-bindgen crate of
# packages/ts/wasm/Cargo.toml (binding GN-15, GN-18); test-go and node-package compare both releases before they use
# them. Both are the ones of mise.toml; make release-npm of the release recipes starts the same wasm-bindgen CLI.
GO ?= $(VAR_TOOLS)/mise/shims/go
GOFMT ?= $(VAR_TOOLS)/mise/shims/gofmt
WASM_BINDGEN ?= $(VAR_TOOLS)/mise/shims/wasm-bindgen
# ext-php-rs reads them when it builds the extension.
export PHP PHP_CONFIG
# The file name extension of a library that Cargo builds as a cdylib: dylib on macOS, so on Linux (AGENTS.md,
# "Platforms").
CDYLIB := $(if $(filter Darwin,$(shell uname -s)),dylib,so)

.PHONY: bootstrap install-tools install-packages install-php-tools toolchain-lock install-rust ci-bindings install-bay bay-path setup-bay install-bay-test ci-shared ci-rust ci-test ci-assets-platform ci-assets-php ci-assets-npm ci-release-tag ci-release-release ci-passed owner-check owner-validate docs-check documents-stamp workflows-test tool-pins-test servers-test redis-state-test install-bay-test commit-names-check commit-names-test check test redis-image lint-rust fmt-check binding-check bindings $(addprefix bindings-,$(TARGETS)) test-bindings lint-bindings check-declarations \
	test-python node-package test-node test-wasm test-php test-go \
	release-coverage release-tag release-tag-output release-tools release-tools-path release-assets release-publish release-undraft \
	release-recipes

# The targets of each job of .github/workflows/ci.yml, which ci-targets of kit runs past failures into CI_REPORT. The
# job test runs make test after its prerequisites (ci-test).
CI_SHARED := docs-check owner-validate workflows-test tool-pins-test servers-test redis-state-test install-bay-test commit-names-check commit-names-test release-coverage
CI_RUST := lint-rust fmt-check binding-check
# The tests of the bindings of every target of the contract (binding BD-27), which the job bindings runs, so that no
# binding package is released untested.
CI_BINDINGS := lint-bindings check-declarations test-python test-node test-wasm test-php test-go
CI_REPORT ?= var/report/ci-targets

check: lint-rust fmt-check test binding-check test-bindings docs-check owner-validate workflows-test tool-pins-test servers-test redis-state-test install-bay-test commit-names-check commit-names-test release-coverage

# The npm that devEngines of package.json names installs the lock, @polyspec/kit included, before the tools of the
# checkout exist, since install-tools TOOLS=npm runs the bootstrap of the installed kit. It runs on the node of the
# machine, outside the checkout with --prefix, since an npm of another release refuses a folder whose devEngines names
# npm 12.2.0 (EBADDEVENGINES); npm reads allow-remote of the .npmrc of the checkout. A fresh checkout runs make
# bootstrap, make install-tools TOOLS=npm and make install-packages, in that order.
BOOTSTRAP_NPM := $(abspath $(shell command -v npm))
bootstrap:
	root="$$PWD"; version="$$(node -p 'require("./package.json").devEngines.packageManager.version')"; \
	cd "$${TMPDIR:-/tmp}" && npm_config_cache="$$root/var/bootstrap/npm-cache" "$(BOOTSTRAP_NPM)" exec --yes --package="npm@$$version" -- npm --prefix "$$root" ci --ignore-scripts

# TOOLS=npm runs the bootstrap of kit on the node of the machine, which only starts it: it installs the mise of
# config/toolchain.json by its sha256, then with it the node and the npm of mise.toml at the digests of mise.lock, into
# var/tools. Any other TOOLS names tools of mise.toml; without TOOLS every tool is installed.
install-tools:
ifeq ($(TOOLS),npm)
	node $(KIT)/bootstrap/bootstrap.mjs
else
	$(NODE) $(KIT)/install-tools.mjs --no-self-update $(TOOLS)
endif

# The packages of package-lock.json with the npm of var/tools, which reads .npmrc.
install-packages:
	$(NPM) ci

# The PHPUnit of composer.lock into vendor, with the Composer of mise.toml; the one target that installs PHP packages.
install-php-tools:
	$(COMPOSER) install --no-interaction --no-progress

# Writes mise.lock with mise lock for each platform that has the archives of the tools.
TOOLCHAIN_LOCK_PLATFORMS := linux-x64,linux-arm64,macos-arm64,macos-x64,windows-x64,windows-arm64
toolchain-lock:
	@test -x $(MISE) || { echo "toolchain-lock: $(MISE) is missing; run make install-tools TOOLS=npm" >&2; exit 1; }
	$(MISE) lock --global --platform $(TOOLCHAIN_LOCK_PLATFORMS)

# The toolchain of rust-toolchain.toml, which rustup installs, and the crates of Cargo.lock, for a job of CI.
install-rust:
	rustup toolchain install --no-self-update
	$(CARGO) fetch --locked

# Each job of .github/workflows/ci.yml and assets.yml runs one recipe ci-<job>, with the values of its matrix and of the
# inputs of assets.yml as make variables, so that a local run is the call that CI makes: make ci-<job> NAME=value. A recipe
# runs the installs and the prerequisites of its job first, which stop at a failure, then the checks or the builds of the
# job, several of them with ci-targets past failures into CI_REPORT; its status is that of the checks. The other steps of
# a job only provision the runner (checkout, setup-php, the bootstrap action) or carry a result (upload-artifact). The
# servers of a test run are started and removed by scripts/test-run.mts inside the test target, also when it fails.
ci-shared:
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_SHARED)

ci-rust:
	$(MAKE) --no-print-directory install-rust
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_RUST)

# The job test: the toolchain and the pinned Redis image, then make test. redis-image fails first, naming `bay install`,
# on a machine whose bay or its setup is missing.
ci-test:
	$(MAKE) --no-print-directory install-rust
	$(MAKE) --no-print-directory redis-image
	$(MAKE) --no-print-directory test

# The job bindings: the toolchain, every tool of mise.toml, PHPUnit and the pinned Redis image of the service of the
# generated tests (SE-10.7), then the tests of CI_BINDINGS; redis-image checks bay and its setup as in ci-test.
ci-bindings:
	$(MAKE) --no-print-directory install-rust
	$(MAKE) --no-print-directory install-tools
	$(MAKE) --no-print-directory install-php-tools
	$(MAKE) --no-print-directory redis-image
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_BINDINGS)

# The jobs of assets.yml. Each writes the release recipes, and with RELEASE=true, the input release of assets.yml that the
# workflow release passes, runs the release check of binding before it builds.
release_check = $(if $(filter true,$(RELEASE)),$(MAKE) --no-print-directory release-binding-check)

# The job platform: the static library of the Go module, the wheel and the npm platform package of this host.
ci-assets-platform:
	$(MAKE) --no-print-directory install-rust
	$(MAKE) --no-print-directory install-tools TOOLS="uv python"
	$(MAKE) --no-print-directory release-recipes
	$(release_check)
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) release-go-static release-wheel release-npm-platform

# The job php: the PHP extension for the PHP on PATH, whose minor is EXPECTED_PHP_MINOR. The job provisions the LLVM of
# config/toolchain.json before it (binding GN-19i): the ubuntu-26.04 images hold it at /usr/lib/llvm-<major>, the default
# LLVM_ROOT of packages/release.mk, and on macOS install-llvm-action installs it and sets LLVM_PATH, which the build takes
# as LLVM_ROOT.
ci-assets-php:
	@test -n "$(EXPECTED_PHP_MINOR)" || { echo 'ci-assets-php needs EXPECTED_PHP_MINOR, the PHP minor of the job, for example make ci-assets-php EXPECTED_PHP_MINOR=8.4' >&2; exit 2; }
	$(MAKE) --no-print-directory install-rust
	$(MAKE) --no-print-directory release-recipes
	$(release_check)
	$(MAKE) --no-print-directory release-php-ext PHP=php EXPECTED_PHP_MINOR=$(EXPECTED_PHP_MINOR) $(if $(LLVM_PATH),LLVM_ROOT=$(LLVM_PATH))

# The job npm: the npm package, packed with the wasm-bindgen of mise.toml.
ci-assets-npm:
	$(MAKE) --no-print-directory install-rust
	$(MAKE) --no-print-directory release-recipes
	$(MAKE) --no-print-directory install-wasm-bindgen
	$(release_check)
	$(MAKE) --no-print-directory release-npm

# The last job of .github/workflows/ci.yml: fails unless every job of RESULTS, the JSON of toJSON(needs), has the result
# success; with FAIL_SKIPPED=1 a skipped job fails it too. RESULTS that is not set or names no job fails it with status 2.
ci-passed:
	$(NODE) $(KIT)/ci-passed.mjs

# The documents of config/documents.json, each on its own (links, anchors, fences and the doc-id and source-sha256 of each
# Korean twin), and the checklists of docs/plans, whose rows config/checklist.json declares.
docs-check:
	$(NODE) $(KIT)/check-documents.mjs --checklist-place --private-path /Users/ --private-path /home/ --kit-rows

# Writes the source-sha256 of each English document into the Korean twins that PATHS names, after they are translated
# again; with no PATHS it lists each twin whose source moved.
documents-stamp:
	$(NODE) $(KIT)/documents-stamp.mjs $(PATHS)

# The checks that own the changed paths (config/owner-checks.json): PATHS="a b" or BASE=<revision>.
owner-check:
	$(NODE) $(KIT)/owner-check.mjs --every-path $(if $(PATHS),--paths "$(PATHS)") $(if $(BASE),--base $(BASE))

# config/owner-checks.json owns every tracked path and names only targets that the Makefile defines.
owner-validate:
	$(NODE) $(KIT)/owner-check.mjs --every-path --validate

# The rules of the GitHub workflows (AGENTS.md, "CI and release"), on the installed test runner of kit.
workflows-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/workflows.test.mts

# The pins of the tools of the checkout (AGENTS.md, "Tools"): mise.toml with mise.lock, devEngines of package.json,
# config/toolchain.json, the bootstrap action and the node that the recipes start.
tool-pins-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/tool-pins.test.mts

# The commands of scripts/servers.sh and the run of scripts/test-run.mts (SE-10.4), against a stand-in of bay that the
# test writes, so it needs neither bay nor a container runtime.
servers-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/servers.test.mts

# The program of the state of the Redis service of the generated binding tests (scripts/redis-state.mts, SE-10.7),
# against a stand-in of a Redis server that the test serves, so it needs no Redis server.
redis-state-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/redis-state.test.mts

# The install of the pinned release of bay (scripts/install-bay.mts), with an archive that the test packs, so it
# downloads nothing.
install-bay-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/install-bay.test.mts

# No tracked Markdown file names a Git commit (AGENTS.md, "Commits"); commit-names-test tests the checker.
commit-names-check:
	$(NODE) scripts/check-commit-names.mts

commit-names-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/commit-names.test.mts

# scripts/test-run.mts owns the servers and the temporary directory of the run and removes them when the run ends,
# fails or is stopped by SIGINT or SIGTERM; it writes the version of bay as its first line and fails before the tests
# when bay is missing or lacks a part of its machine setup. PHP that is not found is passed empty, and the test that needs it fails.
test:
	BAY='$(BAY)' $(NODE) scripts/test-run.mts env PHP='$(PHP)' REDIS_IMAGE='$(REDIS_IMAGE)' \
		$(CARGO) test $(CARGOFLAGS) $(CORE) --locked

# Pulls the pinned Redis image of the tests with bay (SE-10.4), the only step that pulls, after the check of the run:
# the version of bay as the first line, and its machine setup.
redis-image:
	BAY='$(BAY)' scripts/servers.sh check
	BAY='$(BAY)' scripts/servers.sh pull '$(REDIS_IMAGE)'

# Installs the release of bay that config/toolchain.json pins for CI into var/tools/bay by the sha256 of its archive,
# for a machine that holds no bay; it changes no setup of the machine, and no run reads that pin.
install-bay:
	$(NODE) scripts/install-bay.mts

# Puts var/tools/bay, where make install-bay installs bay, on the PATH of the later steps of a job of GitHub
# (GITHUB_PATH), so that BAY, bay of PATH, starts it: the action .github/actions/bay runs it.
bay-path:
	@test -n "$$GITHUB_PATH" || { echo 'bay-path: GITHUB_PATH is not set; run it in a job of GitHub' >&2; exit 2; }
	@echo "$(VAR_TOOLS)/bay" >> "$$GITHUB_PATH"

# Applies the machine setup of bay with its install command, once per machine, the one target that changes the setup of a
# machine: the action .github/actions/bay runs it on a runner, before the run step of its job, and a developer runs it by
# choice. No recipe of a job runs it.
setup-bay:
	$(BAY) install

lint-rust:
	$(CARGO) clippy $(CARGOFLAGS) $(CORE) --locked --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt $(CORE) --check

# binding reads the crates with cargo metadata, which Cargo runs offline here, after make install-rust fetched them, so
# that no check depends on the network.
binding-check $(addprefix bindings-,$(TARGETS)): export CARGO_NET_OFFLINE := true

# The contract check and the comparison of the tracked binding packages with what binding writes (CK-19).
binding-check:
	$(BINDING) check packages/rust/interface.json

# Writes the generated sources into the directories generated/ of the binding packages, which are not tracked, and the
# Go files zz_generated_*.go of packages/go, which are tracked (binding GN-3).
# Each target of the contract is generated apart (binding GN-1b), so that a target that the generator refuses fails
# its own make targets alone, with status 1, and the others build and test.
bindings: $(addprefix bindings-,$(TARGETS))

$(addprefix bindings-,$(TARGETS)): bindings-%:
	$(BINDING) generate --root . --target $*

# Every target of the contract (binding BD-27), each with the tests that binding generates and the contract cases of
# the binding on the standard runner of its language.
test-bindings: lint-bindings check-declarations test-python test-node test-wasm test-php test-go

lint-bindings: bindings
	$(CARGO) clippy $(CARGOFLAGS) --workspace --locked --all-targets --features panic-test -- -D warnings

check-declarations: bindings-node bindings-wasm
	$(TSC) --noEmit --strict packages/ts/generated/index.d.ts
	$(TSC) --noEmit --strict packages/ts/wasm/generated/index.d.ts

# The run of the generated binding tests of a target whose core reaches a socket (SE-10.7): scripts/test-run.mts starts
# the Redis service of the cases with a service (binding GN-13j) from the image REDIS_IMAGE with bay, gives the tests
# SESSION_STORE_REDIS_URL and SESSION_STORE_REDIS_STATE, and removes the server when the tests end, fail or are stopped.
# The WASM tests run without it, because their core has no socket and the cases of a service have no WASM test.
SERVICE_RUN = BAY='$(BAY)' REDIS_IMAGE='$(REDIS_IMAGE)' $(NODE) $(CURDIR)/scripts/test-run.mts --service redis

# The wheel that the maturin of mise.toml builds, installed into a virtual environment without an index; the generated
# tests (binding G-5), which run the contract cases.
test-python: bindings-python
	$(PYTHON) -m venv --clear $(VAR)/venv
	rm -rf $(VAR)/wheels
	cd packages/python && $(MATURIN) build --locked --interpreter $(VAR)/venv/bin/python --features panic-test,fault-test --out $(VAR)/wheels $(CARGOFLAGS)
	$(VAR)/venv/bin/pip install -q --no-index --force-reinstall $(VAR)/wheels/*.whl
	cd $(VAR) && $(SERVICE_RUN) venv/bin/python -m unittest discover -s $(CURDIR)/packages/python/generated/tests -t $(CURDIR)/packages/python/generated/tests

# The package and the platform package of the addon, packed and installed into var/node. The addon is the npm package
# of this machine's platform, which the package names in optionalDependencies; the recipe packs it as a release does and
# installs both tarballs, without the other platforms' packages. The package also holds the WASM module and its glue,
# which wasm-bindgen writes into packages/ts/wasm/generated before the package is packed (binding GN-18, P-5).
PLATFORM = $(shell $(NODE) -p 'process.platform + "-" + process.arch + (process.platform === "linux" ? "-gnu" : "")')
NODE_ADDON = session-store.$(PLATFORM).node
WASM_BINDGEN_RELEASE := $(shell sed -n 's/^wasm-bindgen = "=\(.*\)"$$/\1/p' packages/ts/wasm/Cargo.toml)
node-package: bindings-node bindings-wasm
	test "$$($(WASM_BINDGEN) --version)" = "wasm-bindgen $(WASM_BINDGEN_RELEASE)"
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-wasm --locked --target wasm32-unknown-unknown --features panic-test
	$(WASM_BINDGEN) --target web --out-dir packages/ts/wasm/generated --out-name session_store_wasm \
		$(CARGO_TARGET_DIR)/wasm32-unknown-unknown/debug/polyspec_session_store_wasm.wasm
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-node --locked --features panic-test,fault-test
	rm -rf $(VAR)/node $(VAR)/node-platform && mkdir -p $(VAR)/node $(VAR)/node-platform
	cp $(CARGO_TARGET_DIR)/debug/libpolyspec_session_store_node.$(CDYLIB) $(VAR)/node-platform/$(NODE_ADDON)
	$(NODE) -e 'const p = require("./packages/ts/package.json"); require("fs").writeFileSync("$(VAR)/node-platform/package.json", JSON.stringify({ name: p.name + "-$(PLATFORM)", version: p.version, os: [process.platform], cpu: [process.arch], main: "$(NODE_ADDON)", files: ["$(NODE_ADDON)"] }, null, 2) + "\n")'
	cd $(VAR)/node-platform && $(NPM) pack --pack-destination $(VAR)/node
	cd packages/ts && $(NPM) pack --pack-destination $(VAR)/node
	echo '{ "private": true }' > $(VAR)/node/package.json
	cd $(VAR)/node && $(NPM) install --no-audit --no-fund --omit=optional ./*.tgz

# The generated tests of the Node binding, which run the contract cases, on node --test against the installed package.
test-node: node-package
	cp packages/ts/generated/tests/generated.test.mjs $(VAR)/node/
	cd $(VAR)/node && $(SERVICE_RUN) $(NODE) --test generated.test.mjs

# The generated tests of the WASM binding, which run the contract cases, through the subpath ./wasm of the installed package
# (binding P-5); --expose-gc serves the generated tests that collect instances (binding GN-18a).
test-wasm: node-package
	cp packages/ts/wasm/generated/tests/generated.test.mjs $(VAR)/node/generated.wasm.test.mjs
	cd $(VAR)/node && $(NODE) --expose-gc --test generated.wasm.test.mjs

# The extension built against PHP_CONFIG and loaded into PHP; the stub is linted, then the generated tests and the
# contract cases that they hold run on PHPUnit.
test-php: bindings-php
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-php-ext --locked --features panic-test,fault-test
	$(PHP) -l packages/php-ext/generated/polyspec_session_store.stub.php
	$(SERVICE_RUN) $(PHP) -d extension=$(CARGO_TARGET_DIR)/debug/libpolyspec_session_store.$(CDYLIB) $(PHPUNIT) packages/php-ext/generated/tests

# The static library with the test features (binding GN-13b), which Go links through CGO_LDFLAGS; gofmt and go vet,
# then the generated tests, which run the contract cases, on go test. GOTOOLCHAIN=local: a go of another
# release than the go directive fails instead of fetching that release.
GO_RELEASE := $(shell sed -n 's/^go \(.*\)$$/\1/p' packages/go/go.mod)
GO_ENV := GOTOOLCHAIN=local CGO_LDFLAGS=-L$(CARGO_TARGET_DIR)/debug GOFLAGS=-tags=polyspec_panic_test,polyspec_codec_test
test-go: bindings-go
	test "$$($(GO_ENV) $(GO) env GOVERSION)" = "go$(GO_RELEASE)"
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-go --locked --features panic-test,codec-test
	test -z "$$(cd packages/go && $(GOFMT) -l .)"
	cd packages/go && $(GO_ENV) $(GO) vet ./...
	cd packages/go && $(SERVICE_RUN) env $(GO_ENV) $(GO) test -count=1 -v ./...

# ---- Release recipes of the bindings (binding GN-19h) ----
# binding release-recipes writes the recipes that build the assets of each platform (the static library of the Go module,
# the abi3 wheel, the PHP extension, the npm package with its platform packages), carry the Go module and upload the
# assets into packages/release.mk, which is generated and not tracked; make release-recipes writes it again from the
# contract, and binding check reports a Makefile that does not include it or that holds a recipe of its own for one of its
# targets (CK-94). The recipes read BINDING_RUN, the command that starts binding, NODE and NPM, defined above, and UV, the
# uv of mise.toml. BINDING_DEP is empty: the bootstrap installs binding from package-lock.json before any recipe runs, and
# a prerequisite that installs it would download in a recipe that builds.
BINDING_RUN := $(BINDING)
BINDING_DEP :=
UV := $(VAR_TOOLS)/mise/shims/uv
release-recipes:
	$(BINDING_RUN) release-recipes --root $(CURDIR)
# The file is missing in a checkout until make release-recipes writes it, so the include is optional, and a run of one of
# its targets without it fails with no rule of that target.
-include packages/release.mk

# ---- Release (.github/workflows/release.yml) ----
# Every manifest of the checkout is classified in config/release.json: released by a tag or not released, with its reason.
release-coverage:
	$(NODE) $(KIT)/release.mjs coverage

# Tags the commit of the successful run RUN of the workflow ci on main with the version of the first section ## X.Y.Z of
# its change log and writes TAG and VERSION for the steps that follow; AUTO for a run that the workflow ci started.
release-tag:
	@test -n "$$RUN" || { echo "release-tag: RUN is required, for example make release-tag RUN=123"; exit 1; }
	RELEASE_MAIN=origin/main $(NODE) $(KIT)/release-run.mjs tag "$$RUN"$(if $(AUTO), --auto)

# Gives the later jobs of the release workflow the tag and the version that release-tag wrote into the environment of
# its job (TAG, VERSION), through the outputs of the step (GITHUB_OUTPUT); both are empty when the run releases nothing.
release-tag-output:
	@test -n "$$GITHUB_OUTPUT" || { echo 'release-tag-output: GITHUB_OUTPUT is set only in a step of a workflow' >&2; exit 2; }
	printf 'tag=%s\nversion=%s\n' "$$TAG" "$$VERSION" >> "$$GITHUB_OUTPUT"

# The jobs of release.yml, each one recipe that its one run step runs (T1.35). ci-release-tag tags the commit of the run
# RUN and gives the later jobs the TAG and VERSION that release-tag appended to GITHUB_ENV, which a later line of the same
# step does not receive in its environment. ci-release-release builds and publishes the release of TAG: the tools of
# release-tools are on PATH for its later lines (RELEASE_PATH, in the order that release-tools-path gives them), the
# artifacts of the job assets, which the workflow downloads into var/release/downloaded before this step, join
# var/release/assets after the draft release exists, and the commit of the Go module carries the identity of
# GITHUB_ACTOR, the actor of the run.
RELEASE_PATH = PATH="$(VAR_TOOLS)/mise/shims:$(VAR_TOOLS)/bin:$$PATH"
RELEASE_IDENTITY = GIT_AUTHOR_NAME="$${GITHUB_ACTOR:?}" GIT_AUTHOR_EMAIL="$${GITHUB_ACTOR:?}@users.noreply.github.com" \
	GIT_COMMITTER_NAME="$${GITHUB_ACTOR:?}" GIT_COMMITTER_EMAIL="$${GITHUB_ACTOR:?}@users.noreply.github.com"
ci-release-tag:
	@test -n "$$GITHUB_ENV" || { echo 'ci-release-tag: GITHUB_ENV is set only in a step of a workflow' >&2; exit 2; }
	$(MAKE) --no-print-directory release-tag
	$(MAKE) --no-print-directory release-tag-output TAG="$$(sed -n 's/^TAG=//p' "$$GITHUB_ENV" | tail -n 1)" VERSION="$$(sed -n 's/^VERSION=//p' "$$GITHUB_ENV" | tail -n 1)"

ci-release-release:
	@test -n "$(TAG)" || { echo 'ci-release-release: TAG is required, for example make ci-release-release TAG=v0.0.1' >&2; exit 2; }
	$(MAKE) --no-print-directory release-tools
	$(RELEASE_PATH) $(MAKE) --no-print-directory release-assets
	$(RELEASE_PATH) $(MAKE) --no-print-directory release-publish
	mkdir -p var/release/assets && cp var/release/downloaded/* var/release/assets/
	$(RELEASE_PATH) $(MAKE) --no-print-directory install-rust
	$(RELEASE_PATH) $(MAKE) --no-print-directory release-recipes
	$(RELEASE_IDENTITY) $(RELEASE_PATH) $(MAKE) --no-print-directory release-go-module
	$(RELEASE_PATH) $(MAKE) --no-print-directory release-platform-assets
	$(RELEASE_PATH) $(MAKE) --no-print-directory release-undraft

# The tools that the packages of config/release.json need: the Composer package of the PHP extension needs the tools of
# mise.toml, which install-tools of kit installs into var/tools.
release-tools:
	$(NODE) $(KIT)/release-tools.mjs

# Puts var/tools/bin and the shims of mise on the PATH of the later steps of a workflow, with the folders of mise, so that
# a step finds the tools that release-tools installed.
release-tools-path:
	@test -n "$$GITHUB_PATH" && test -n "$$GITHUB_ENV" || { echo 'release-tools-path: GITHUB_PATH and GITHUB_ENV are set only between the steps of a workflow' >&2; exit 2; }
	printf '%s\n%s\n' "$(VAR_TOOLS)/bin" "$(VAR_TOOLS)/mise/shims" >> "$$GITHUB_PATH"
	printf 'MISE_DATA_DIR=%s\nMISE_CACHE_DIR=%s\nMISE_STATE_DIR=%s\nMISE_CONFIG_DIR=%s\nMISE_GLOBAL_CONFIG_FILE=%s\nMISE_GLOBAL_CONFIG_ROOT=%s\n' \
		"$(MISE_DATA_DIR)" "$(MISE_CACHE_DIR)" "$(MISE_STATE_DIR)" "$(MISE_CONFIG_DIR)" "$(MISE_GLOBAL_CONFIG_FILE)" "$(MISE_GLOBAL_CONFIG_ROOT)" >> "$$GITHUB_ENV"

# The archive of every package of config/release.json at TAG, into var/release/assets.
release-assets:
	@test -n "$$TAG" || { echo "release-assets: TAG is required, for example make release-assets TAG=v0.0.1"; exit 1; }
	RELEASE_ASSETS=var/release/assets $(NODE) $(KIT)/release.mjs assets "$$TAG"

# The GitHub Release of TAG as a draft, with its notes and the files of release-assets.
release-publish:
	@test -n "$$TAG" || { echo "release-publish: TAG is required, for example make release-publish TAG=v0.0.1"; exit 1; }
	RELEASE_ASSETS=var/release/assets RELEASE_RECORDS=var/release $(NODE) $(KIT)/release.mjs publish "$$TAG"

# Makes the draft release of TAG public.
release-undraft:
	@test -n "$$TAG" || { echo "release-undraft: TAG is required, for example make release-undraft TAG=v0.0.1"; exit 1; }
	RELEASE_RECORDS=var/release $(NODE) $(KIT)/release.mjs undraft "$$TAG"
