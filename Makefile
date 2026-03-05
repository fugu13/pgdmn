.PHONY: base-image test-image check test clean

DOCKER_RUN = docker run --rm -e USER=pgdmn -v "$$(pwd)":/pgdmn -w /pgdmn pgdmn-test

# Build the base Docker image (PG17 + pgrx toolchain)
base-image:
	docker build -t pgdmn-base .

# Build the test image (adds non-root user required by initdb)
test-image: base-image
	printf 'FROM pgdmn-base\nRUN useradd -ms /bin/bash pgdmn\nUSER pgdmn\n' | docker build -t pgdmn-test --build-arg BASE=pgdmn-base -f - .

# Run cargo check (fast compilation check, no tests)
check:
	$(DOCKER_RUN) cargo check

# Run the pgrx test suite against PG17
test:
	$(DOCKER_RUN) cargo pgrx test pg17

# Remove build artifacts
clean:
	cargo clean
