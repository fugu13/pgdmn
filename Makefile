.PHONY: base-image test-image check test bench clean website website-dev website-build website-serve

DOCKER_RUN = docker run --rm -e USER=pgdmn -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test

# Build the base Docker image (PG17 + pgrx toolchain)
base-image:
	docker build -t pgdmn-base .

# Build the test image (adds non-root user required by initdb)
test-image: base-image
	printf 'FROM pgdmn-base\nRUN useradd -ms /bin/bash pgdmn\nUSER pgdmn\n' | docker build -t pgdmn-test -f - .

# Run cargo check (fast compilation check, no tests)
check: test-image
	$(DOCKER_RUN) cargo check

# Run the pgrx test suite against PG17
test: test-image
	$(DOCKER_RUN) cargo pgrx test pg17

# Run DMN eval benchmark and print results (gated by PGDMN_BENCH=1)
bench: test-image
	docker run --rm -e USER=pgdmn -e PGDMN_BENCH=1 -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test cargo pgrx test pg17 -- bench_dmn_eval_vs_pg_concat
	@cat benchmark_results.txt 2>/dev/null

# Remove build artifacts
clean:
	rm -rf target/

# Run the website dev server with hot-reload
website-dev:
	cd website && cargo leptos watch

# Build the website for production
website-build:
	cd website && cargo leptos build --release

# Serve the production build
website-serve:
	cd website && ./target/release/pgdmn-website

# Open the website in the browser and start the dev server
website:
	open http://127.0.0.1:3000
	cd website && cargo leptos watch
