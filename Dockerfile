# 1) Build stage
FROM rust:1.88-bullseye AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source so Cargo has a target
RUN mkdir src && echo "fn main() {}" > src/main.rs

# 🔥 Force rebuild when dependencies/features change
RUN echo "rebuild deps"

# Build dependencies (this layer caches correctly BUT now refreshes when needed)
RUN cargo build --release

# Remove dummy source
RUN rm -rf src

# Copy real source
COPY src ./src

# Build actual app (will link with correct TLS-enabled sqlx)
RUN cargo build --release


# 2) Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install SSL libs (needed for TLS at runtime)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary
COPY --from=builder /app/target/release/axum_backend /usr/local/bin/axum_backend

EXPOSE 3000

CMD ["axum_backend"]
