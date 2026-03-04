FROM rust:1.85-bookworm

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

# Initialize pgrx for PG17 using the system installation
RUN cargo pgrx init --pg17=/usr/lib/postgresql/17/bin/pg_config

WORKDIR /pgdmn

# Copy manifests first for layer caching
COPY Cargo.toml pgdmn.control ./

# Create stub lib.rs so cargo fetch works
RUN mkdir -p src && echo '::pgrx::pg_module_magic!();' > src/lib.rs

# Fetch dependencies (cached layer)
RUN cargo fetch

# Copy actual source
COPY src/ src/

# Default: build and test
CMD ["cargo", "pgrx", "test", "pg17"]
