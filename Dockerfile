# dsntk 0.3 uses let-chains (stable since Rust 1.88)
FROM rust:1.97-bookworm

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

# pgrx tests must run as a non-root user (initdb refuses root). Create that
# user and hand it the PG install dirs and its own CARGO_HOME up front, so
# every later step (toolchain components, crate fetch, pgrx init) is owned
# by pgdmn by construction—see BUG-001 and BUG-002 in BUGHISTORY.md.
RUN useradd -ms /bin/bash pgdmn \
    && chown -R pgdmn /usr/share/postgresql/17/extension /usr/lib/postgresql/17/lib
ENV CARGO_HOME=/home/pgdmn/.cargo \
    PATH=/home/pgdmn/.cargo/bin:$PATH
USER pgdmn
WORKDIR /pgdmn

# Install cargo-pgrx matching our dependency version
RUN cargo install cargo-pgrx --version "~0.16" --locked

# Lint and format tooling for `make lint` / `make fmt`
RUN rustup component add clippy rustfmt

# pgrx init reads nothing from the project manifests (its state lives in
# ~/.pgrx), so it sits above the COPY to stay cached across dependency bumps
RUN cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config

# Copy manifests first for layer caching; vendor/ holds the patched dsntk
# sources referenced by [patch.crates-io], needed for dependency resolution
COPY --chown=pgdmn:pgdmn Cargo.toml Cargo.lock pgdmn.control ./
COPY --chown=pgdmn:pgdmn vendor ./vendor

# Create stub lib.rs so cargo fetch works
RUN mkdir -p src && echo '::pgrx::pg_module_magic!();' > src/lib.rs

# Fetch dependencies into pgdmn's own registry cache
RUN cargo fetch --locked

# Default: build and test (the repo is bind-mounted at /pgdmn by `make`)
CMD ["cargo", "pgrx", "test", "pg17"]
