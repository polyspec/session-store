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
# binding at the commit of Cargo.lock (scripts/binding.mts).
BINDING := $(NODE) scripts/binding.mts
# The real peers of the tests, each required: PHP reads the records back (SE-10.3), and bay runs the Redis servers of the
# stores of Redis from the image REDIS_IMAGE as Compose projects, on Apple container on Darwin and on Docker on Linux,
# whose commands scripts/servers.sh holds, and issues their certificates of TLS (SE-10.4). A test whose peer is not
# given fails and names what gives it.
PHP ?= $(shell command -v php)
# How bay is started, the one value that scripts/servers.sh reads (SE-10.4): bay of PATH, which the owner of a
# development machine installed and set up, or with BAY_SUDO=1 the bay that make install-bay put into var/tools, as
# root without a question for a password. The workflow ci sets BAY_SUDO=1: on Linux the setup and the commands of bay
# run as root, and bay keeps its state below the home of the user who runs it, so every command runs as one user.
BAY ?= $(if $(BAY_SUDO),sudo -n $(VAR_TOOLS)/bay/bay,bay)
# The version of bay that config/toolchain.json pins and the address of that release. A run fails at its start when
# the bay that it starts gives another version.
BAY_PIN = $(shell $(NODE) -p 'const bay = require("./config/toolchain.json").bay; bay.version + " " + bay.releases + "/tag/" + bay.version')
BAY_ENV = BAY='$(BAY)' BAY_VERSION='$(word 1,$(BAY_PIN))' BAY_RELEASE='$(word 2,$(BAY_PIN))'
# Redis 8.10.3, the highest stable release of Redis, in the official image of Debian trixie, pinned by the digest of its
# index; make redis-image pulls it, and the tests fail while it is not installed, so that no test pulls an image.
REDIS_IMAGE := docker.io/library/redis:8.10.3-trixie@sha256:de1068af6f552240dfab67bcc9bfef99bfc60f776bb4580ef077ea4c8c97e848
# The directory of the cases (SE-10), which the contract case runners of the bindings read.
CASES := $(CURDIR)/contracts
VAR := $(CURDIR)/var
export CARGO_TARGET_DIR ?= $(CURDIR)/target
# The runtimes and tools of the bindings. PHP is also the binary whose headers the extension is built against
# (PHP_CONFIG) and that loads it; both are paths, because ext-php-rs reads PHP as the path of a file. TSC checks the
# generated declarations and PHPUNIT runs the PHP tests.
PYTHON ?= python3.14
PHP_CONFIG ?= $(shell command -v php-config)
PHPUNIT ?= phpunit
TSC ?= tsc
MATURIN := maturin==1.16.0
# The Go of the go directive of packages/go/go.mod and the wasm-bindgen CLI of the release of the wasm-bindgen crate of
# packages/ts/wasm/Cargo.toml (binding GN-15, GN-18); test-go and node-package compare both releases before they use
# them.
GO ?= go
GOFMT ?= gofmt
WASM_BINDGEN ?= wasm-bindgen
# ext-php-rs reads them when it builds the extension.
export PHP PHP_CONFIG
# The file name extension of a library that Cargo builds as a cdylib: dylib on macOS, so on Linux (AGENTS.md,
# "Platforms").
CDYLIB := $(if $(filter Darwin,$(shell uname -s)),dylib,so)

.PHONY: bootstrap install-tools install-packages toolchain-lock install-rust install-bay setup-bay install-bay-test ci-shared ci-rust ci-passed owner-check owner-validate docs-check documents-stamp workflows-test tool-pins-test servers-test install-bay-test commit-names-check commit-names-test check test redis-image lint-rust fmt-check binding-check bindings $(addprefix bindings-,$(TARGETS)) test-bindings lint-bindings check-declarations \
	test-python node-package test-node test-wasm test-php test-go \
	release-coverage release-tag release-checkout release-tools release-assets release-publish release-undraft

# The targets of each job of .github/workflows/ci.yml, which ci-targets of kit runs past failures into CI_REPORT. The
# job test runs make redis-image and make test, one target each.
CI_SHARED := docs-check owner-validate workflows-test tool-pins-test servers-test install-bay-test commit-names-check commit-names-test release-coverage
CI_RUST := lint-rust fmt-check binding-check
CI_REPORT ?= var/report/ci-targets

check: lint-rust fmt-check test binding-check docs-check owner-validate workflows-test tool-pins-test servers-test install-bay-test commit-names-check commit-names-test release-coverage

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

# Writes mise.lock with mise lock for each platform that has the archives of the tools.
TOOLCHAIN_LOCK_PLATFORMS := linux-x64,linux-arm64,macos-arm64,macos-x64,windows-x64,windows-arm64
toolchain-lock:
	@test -x $(MISE) || { echo "toolchain-lock: $(MISE) is missing; run make install-tools TOOLS=npm" >&2; exit 1; }
	$(MISE) lock --global --platform $(TOOLCHAIN_LOCK_PLATFORMS)

# The toolchain of rust-toolchain.toml, which rustup installs, and the crates of Cargo.lock, for a job of CI.
install-rust:
	rustup toolchain install --no-self-update
	$(CARGO) fetch --locked

ci-shared:
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_SHARED)

ci-rust:
	$(NODE) $(KIT)/ci-targets.mjs -k $(CI_REPORT) $(CI_RUST)

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

# The commands of scripts/servers.sh and the run of scripts/test-run.sh (SE-10.4), against a stand-in of bay that the
# test writes, so it needs neither bay nor a container runtime.
servers-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/servers.test.mts

# The install of the pinned release of bay (scripts/install-bay.mts), with an archive that the test packs, so it
# downloads nothing.
install-bay-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/install-bay.test.mts

# No tracked Markdown file names a Git commit (AGENTS.md, "Commits"); commit-names-test tests the checker.
commit-names-check:
	$(NODE) scripts/check-commit-names.mts

commit-names-test:
	$(NODE) $(KIT)/run-tests.mjs node -- --test-concurrency=1 scripts/commit-names.test.mts

# scripts/test-run.sh owns the servers and the temporary directory of the run and removes them when the run ends,
# fails or is stopped by SIGINT or SIGTERM; it fails before the tests when bay is missing, is not the pinned release
# or lacks a part of its machine setup. PHP that is not found is passed empty, and the test that needs it fails.
test:
	$(BAY_ENV) scripts/test-run.sh env PHP='$(PHP)' REDIS_IMAGE='$(REDIS_IMAGE)' \
		$(CARGO) test $(CARGOFLAGS) $(CORE) --locked

# Pulls the pinned Redis image of the tests with bay (SE-10.4), the only step that pulls, after the check of the run:
# the pinned release of bay with its machine setup.
redis-image:
	$(BAY_ENV) scripts/servers.sh check
	$(BAY_ENV) scripts/servers.sh pull '$(REDIS_IMAGE)'

# Installs the pinned release of bay of config/toolchain.json into var/tools/bay by the sha256 of its archive, for a
# machine that holds no bay; it changes no setup of the machine.
install-bay:
	$(NODE) scripts/install-bay.mts

# Applies the machine setup of bay with its install command, the one target that changes the setup of a machine: the
# workflow ci runs it on its runner, and a developer runs it by choice.
setup-bay:
	$(BAY) install

lint-rust:
	$(CARGO) clippy $(CARGOFLAGS) $(CORE) --locked --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt $(CORE) --check

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

# The wheel, installed into a virtual environment; the generated tests (binding G-5) and the contract cases (SE-10).
test-python: bindings-python
	$(PYTHON) -m venv --clear $(VAR)/venv
	$(VAR)/venv/bin/pip install -q $(MATURIN)
	rm -rf $(VAR)/wheels
	cd packages/python && $(VAR)/venv/bin/maturin build --locked --features panic-test,fault-test --out $(VAR)/wheels $(CARGOFLAGS)
	$(VAR)/venv/bin/pip install -q --force-reinstall $(VAR)/wheels/*.whl
	cd $(VAR) && venv/bin/python -m unittest discover -s $(CURDIR)/packages/python/generated/tests -t $(CURDIR)/packages/python/generated/tests
	cd $(VAR) && SESSION_STORE_CASES=$(CASES) venv/bin/python -m unittest discover -s $(CURDIR)/packages/python/tests -t $(CURDIR)/packages/python/tests

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

# The generated tests and the contract cases of the Node binding on node --test, against the installed package.
test-node: node-package
	cp packages/ts/generated/tests/generated.test.mjs packages/ts/tests/cases.test.mjs $(VAR)/node/
	cd $(VAR)/node && SESSION_STORE_CASES=$(CASES) $(NODE) --test generated.test.mjs cases.test.mjs

# The generated tests and the contract cases of the WASM binding through the subpath ./wasm of the installed package
# (binding P-5); --expose-gc serves the generated tests that collect instances (binding GN-18a).
test-wasm: node-package
	cp packages/ts/wasm/generated/tests/generated.test.mjs $(VAR)/node/generated.wasm.test.mjs
	cp packages/ts/tests/cases.wasm.test.mjs $(VAR)/node/
	cd $(VAR)/node && SESSION_STORE_CASES=$(CASES) $(NODE) --expose-gc --test generated.wasm.test.mjs cases.wasm.test.mjs

# The extension built against PHP_CONFIG and loaded into PHP; the stub is linted, then the generated tests and the
# contract cases run on PHPUnit.
test-php: bindings-php
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-php-ext --locked --features panic-test,fault-test
	$(PHP) -l packages/php-ext/generated/polyspec_session_store.stub.php
	$(PHP) -d extension=$(CARGO_TARGET_DIR)/debug/libpolyspec_session_store.$(CDYLIB) $(PHPUNIT) packages/php-ext/generated/tests
	SESSION_STORE_CASES=$(CASES) $(PHP) -d extension=$(CARGO_TARGET_DIR)/debug/libpolyspec_session_store.$(CDYLIB) $(PHPUNIT) packages/php-ext/tests

# The static library with the test features (binding GN-13b), which Go links through CGO_LDFLAGS; gofmt and go vet,
# then the generated tests and the contract cases of packages/go/tests on go test. GOTOOLCHAIN=local: a go of another
# release than the go directive fails instead of fetching that release.
GO_RELEASE := $(shell sed -n 's/^go \(.*\)$$/\1/p' packages/go/go.mod)
GO_ENV := GOTOOLCHAIN=local CGO_LDFLAGS=-L$(CARGO_TARGET_DIR)/debug GOFLAGS=-tags=polyspec_panic_test,polyspec_codec_test
test-go: bindings-go
	test "$$($(GO_ENV) $(GO) env GOVERSION)" = "go$(GO_RELEASE)"
	$(CARGO) build $(CARGOFLAGS) -p $(LIBRARY)-go --locked --features panic-test,codec-test
	test -z "$$(cd packages/go && $(GOFMT) -l .)"
	cd packages/go && $(GO_ENV) $(GO) vet ./...
	cd packages/go && $(GO_ENV) SESSION_STORE_CASES=$(CASES) $(GO) test -count=1 -v ./...

# ---- Release (.github/workflows/release.yml) ----
# Every manifest of the checkout is classified in config/release.json: released by a tag or not released, with its reason.
release-coverage:
	$(NODE) $(KIT)/release.mjs coverage

# Tags the commit of the successful run RUN of the workflow ci on main with the version of the first section ## X.Y.Z of
# its change log and writes TAG and VERSION for the steps that follow; AUTO for a run that the workflow ci started.
release-tag:
	@test -n "$$RUN" || { echo "release-tag: RUN is required, for example make release-tag RUN=123"; exit 1; }
	RELEASE_MAIN=origin/main $(NODE) $(KIT)/release-run.mjs tag "$$RUN"$(if $(AUTO), --auto)

# Checks out the tag TAG, so that the release builds the tagged commit with its own kit.
release-checkout:
	@test -n "$$TAG" || { echo "release-checkout: TAG is required, for example make release-checkout TAG=v0.0.1"; exit 1; }
	git checkout --quiet "$$TAG"

# The tools that the packages of config/release.json need; none while it names no package.
release-tools:
	$(NODE) $(KIT)/release-tools.mjs

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
