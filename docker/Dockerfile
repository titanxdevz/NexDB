# Multi-stage build
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /usr/src/nexdb

# Copy Cargo files and source code
COPY nexdb-core/Cargo.toml nexdb-core/Cargo.lock ./nexdb-core/
COPY nexdb-core/lib.rs nexdb-core/main.rs nexdb-core/wal.rs nexdb-core/db.rs nexdb-core/server.rs nexdb-core/document.rs nexdb-core/error.rs nexdb-core/index.rs nexdb-core/query.rs nexdb-core/migrate.rs ./nexdb-core/

# Compile nexdb
WORKDIR /usr/src/nexdb/nexdb-core
RUN cargo build --release

# Final runtime image
FROM debian:bookworm-slim

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/nexdb/nexdb-core/target/release/nexdb /app/nexdb

# Expose default port
EXPOSE 27017

# Volume for database storage
VOLUME ["/data"]

# Run the server
ENTRYPOINT ["/app/nexdb", "serve", "/data/database.nexdb", "--port", "27017"]
