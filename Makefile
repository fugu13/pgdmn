.PHONY: help base-image test-image check build test bench lint fmt verify clean website website-dev website-build website-serve website-lint website-fmt website-clean

DOCKER_RUN = docker run --rm -e USER=pgdmn -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test

# Shared cargo target dir so worktrees reuse the main repo's build cache
REPO_ROOT = $(shell cd "$$(git rev-parse --git-common-dir)/.." && pwd)
WEBSITE_TARGET_DIR = $(REPO_ROOT)/website/target
WEBSITE_CARGO = cd website && CARGO_TARGET_DIR=$(WEBSITE_TARGET_DIR) cargo

help: ## Show available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "%-16s %s\n", $$1, $$2}'

base-image: ## Build the base Docker image (PG17 + pgrx toolchain)
	docker build --target base -t pgdmn-base .

test-image: base-image ## Build the test image (adds non-root user required by initdb)
	docker build --target test -t pgdmn-test .

check: test-image ## Run cargo check (fast compilation check, no tests)
	$(DOCKER_RUN) cargo check --all-targets

build: test-image ## Build the extension
	$(DOCKER_RUN) cargo build

test: test-image ## Run the pgrx test suite against PG17
	$(DOCKER_RUN) cargo pgrx test pg17

bench: test-image ## Run DMN eval benchmark and print results (gated by PGDMN_BENCH=1)
	docker run --rm -e USER=pgdmn -e PGDMN_BENCH=1 -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test cargo pgrx test pg17 -- bench_dmn_eval_vs_pg_concat
	@cat benchmark_results.txt 2>/dev/null

lint: test-image ## Run clippy (deny warnings) and rustfmt check
	$(DOCKER_RUN) sh -c 'cargo clippy --all-targets -- -D warnings && cargo fmt -- --check'

fmt: test-image ## Auto-format code
	$(DOCKER_RUN) cargo fmt

verify: fmt lint ## Run after code changes: fmt + lint (clippy --all-targets subsumes check)

clean: ## Remove build artifacts
	rm -rf target/

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
