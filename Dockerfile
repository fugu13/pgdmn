FROM rust:1.85-bookworm AS base

# Install PostgreSQL 17 and build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-common gnupg2 \
    && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
    && apt-get update && apt-get install -y --no-install-recommends \
    postgresql-17 \
    postgresql-server-dev-17 \
    libclang-dev \
    clang \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-pgrx matching our dependency version
RUN cargo install cargo-pgrx --version "~0.16" --locked

# Lint and format tooling for `make lint` / `make fmt`
RUN rustup component add clippy rustfmt

WORKDIR /pgdmn

# Copy manifests first for layer caching (lockfile keeps the registry cache
# aligned with the versions the repo pins, so the non-root user never needs
# to write to the root-owned registry at runtime — BUG-001 in BUGHISTORY.md)
COPY Cargo.toml Cargo.lock pgdmn.control ./

# Create stub lib.rs so cargo fetch works
RUN mkdir -p src && echo '::pgrx::pg_module_magic!();' > src/lib.rs

# Fetch dependencies; crates fetched as root can extract with owner-only
# modes, so open the registry to the non-root test user in the same layer
# (a separate chmod layer would duplicate the whole registry via copy-up)
RUN cargo fetch --locked && chmod -R a+rwX /usr/local/cargo/registry

# Test image: pgrx tests must run as a non-root user (initdb refuses root).
# That user needs its own ~/.pgrx and write access to the system PG dirs the
# test harness installs the extension into — BUG-002 in BUGHISTORY.md.
FROM base AS test
RUN useradd -ms /bin/bash pgdmn \
    && chown -R pgdmn /usr/share/postgresql/17/extension /usr/lib/postgresql/17/lib
USER pgdmn
RUN cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config

# Default: build and test (the repo is bind-mounted at /pgdmn by `make`)
CMD ["cargo", "pgrx", "test", "pg17"]
