.PHONY: help test-image check build test bench lint fmt verify clean website website-dev website-build website-serve website-lint website-fmt website-clean vendor-status vendor-diff vendor-test vendor-bench vendor-check vendor-upgrade vendor-inspect

DOCKER_RUN = docker run --rm -e USER=pgdmn -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test

# Extra flags for the image build. Empty locally, so `test-image` just uses
# Docker's own layer cache. CI sets this to a GitHub Actions layer cache
# (`--cache-from`/`--cache-to type=gha`) so the ~7-minute image -- apt
# PostgreSQL 17 plus `cargo install cargo-pgrx` -- is reused across runs
# instead of rebuilt every time. buildx (the default builder in modern Docker)
# is what supports these flags and `--load`.
DOCKER_BUILD_CACHE ?=

# Shared cargo target dir so worktrees reuse the main repo's build cache
REPO_ROOT = $(shell cd "$$(git rev-parse --git-common-dir)/.." && pwd)
WEBSITE_TARGET_DIR = $(REPO_ROOT)/website/target
WEBSITE_CARGO = cd website && CARGO_TARGET_DIR=$(WEBSITE_TARGET_DIR) cargo

help: ## Show available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "%-16s %s\n", $$1, $$2}'

test-image: ## Build the Docker image (PG17 + pgrx toolchain, non-root pgdmn user)
	docker buildx build $(DOCKER_BUILD_CACHE) --load -t pgdmn-test .

check: test-image ## Run cargo check (fast compilation check, no tests)
	$(DOCKER_RUN) cargo check --all-targets

build: test-image ## Build the extension
	$(DOCKER_RUN) cargo build

test: test-image ## Run the pgrx test suite against PG17
	$(DOCKER_RUN) cargo pgrx test pg17

bench: test-image ## Run DMN eval benchmark and print results (gated by PGDMN_BENCH=1)
	docker run --rm -e USER=pgdmn -e PGDMN_BENCH=1 -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test cargo pgrx test pg17 -- bench_dmn_eval_vs_pg_concat
	@cat benchmark_results.txt 2>/dev/null

lint: test-image vendor-check ## Run clippy (deny warnings), rustfmt check, and vendor integrity
	$(DOCKER_RUN) sh -c 'cargo clippy --all-targets -- -D warnings && cargo fmt -- --check'

fmt: test-image ## Auto-format code
	$(DOCKER_RUN) cargo fmt

verify: fmt lint vendor-check ## Run after code changes: fmt + lint + vendor integrity (clippy subsumes check)

clean: ## Remove build artifacts
	rm -rf target/

# --- Vendored dsntk management ------------------------------------------
# The git history under vendor/ is a pristine upstream base plus a minimal
# patch layer (one commit per change, PGDMN: markers). See vendor/README.md
# and the Performance section of CLAUDE.md. scripts/vendor.sh implements the mechanics.

# Most recent commit that swapped in a pristine upstream tree, identified by
# its subject line ("Vendor pristine dsntk X" from vendor-upgrade, or the
# original migration merge). Body text mentioning "pristine" must not match.
VENDOR_PRISTINE ?= $(shell git log --format='%H;%s' -- vendor | awk -F';' '$$2 ~ /^Vendor pristine|vendor becomes pristine/ {print $$1; exit}')
# Upstream tests skipped for environmental reasons (one line per skip):
#   external_functions  - requires a live local Java RPC evaluator service (also
#                         compiled out of default builds by DEPS-001)
#   bif_now             - asserts against the wall clock; flaky by construction
#   dmn_3_0076          - TCK model invoking external Java functions (same service)
#   dmn_3_0103::_0017   - asserts a local-timezone-dependent date-time rendering
VENDOR_SKIPS = --skip external_functions --skip bif_now --skip dmn_3_0076 --skip dmn_3_0103::_0017
VENDOR_TEST_PKGS = -p dsntk-common -p dsntk-feel -p dsntk-feel-number -p dsntk-feel-parser -p dsntk-feel-evaluator -p dsntk-model -p dsntk-model-evaluator

vendor-status: ## Show vendored dsntk version, pristine base, and patch-layer size
	@scripts/vendor.sh status "$(VENDOR_PRISTINE)"

vendor-diff: ## Diff vendor/ against the pristine base (the carried patch layer)
	@git diff $(VENDOR_PRISTINE) -- vendor/ ':(exclude)vendor/README.md' ':(exclude)vendor/PATCHES.md' ':(exclude)vendor/rustfmt.toml' ':(exclude)vendor/LICENSE-*' ':(exclude)vendor/NOTICE'

vendor-test: test-image ## Run the vendored engine test suites (env-dependent upstream tests skipped)
	$(DOCKER_RUN) cargo test --no-fail-fast $(VENDOR_TEST_PKGS) -- $(VENDOR_SKIPS)

# On the maintainer's machine the default host toolchain is x86_64-under-Rosetta
# and unusable for measurement; override VENDOR_BENCH_TOOLCHAIN elsewhere.
VENDOR_BENCH_TOOLCHAIN ?= stable-aarch64-apple-darwin

vendor-bench: ## Host-native engine benchmarks over the vendored code (canary methodology: CLAUDE.md Performance section)
	cd profiling && cargo +$(VENDOR_BENCH_TOOLCHAIN) build --release && ./target/release/pgdmn-profiling --samples 30

vendor-check: ## Fail if any dsntk crate resolves from the registry (silent unvendoring) or versions skew
	@scripts/vendor.sh check

vendor-upgrade: ## Stage a new pristine upstream version (VERSION=x.y.z): download, verify, swap, commit
	@test -n "$(VERSION)" || { echo "usage: make vendor-upgrade VERSION=x.y.z"; exit 1; }
	@scripts/vendor.sh upgrade "$(VERSION)"

vendor-inspect: ## Kick off a Claude session that audits upstream changes and re-layers the patch set (after vendor-upgrade)
	claude "$$(cat scripts/vendor-inspect-prompt.md)"

website-clean: ## Remove website build artifacts
	rm -rf $(WEBSITE_TARGET_DIR) website/dist

website-build: ## Prerender the website to website/dist (the deployable artifact)
	$(WEBSITE_CARGO) run --release --bin prerender

website-serve: ## Serve the prerendered site as a static host would
	$(WEBSITE_CARGO) run --release --bin serve

website-dev: website-build ## Prerender, then serve; re-run to pick up changes
	$(MAKE) website-serve

website-lint: ## Run clippy (deny warnings) and rustfmt check on the website
	$(WEBSITE_CARGO) clippy --all-targets -- -D warnings
	$(WEBSITE_CARGO) fmt -- --check

website-fmt: ## Auto-format the website
	$(WEBSITE_CARGO) fmt

website: website-build ## Prerender, open the site in the browser, and serve it
	open http://127.0.0.1:3000
	$(MAKE) website-serve
