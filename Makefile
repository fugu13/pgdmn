.PHONY: base-image test-image check test bench clean

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

# Run DMN eval benchmark and print results
bench: test-image
	$(DOCKER_RUN) cargo pgrx test pg17 -- bench_dmn_eval_vs_pg_concat
	@cat benchmark_results.txt 2>/dev/null

# Remove build artifacts
clean:
	rm -rf target/
