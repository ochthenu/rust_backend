# 1) Build stage
FROM rust:1.88-bullseye AS builder

WORKDIR /app

# Copy everything (this intentionally disables dependency caching to avoid SQLx TLS issues)
COPY . .

# Clean build to guarantee fresh compile with TLS features
RUN cargo clean
RUN cargo build --release


# 2) Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install SSL libs (required for TLS)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary
COPY --from=builder /app/target/release/axum_backend /usr/local/bin/axum_backend

EXPOSE 3000

CMD ["axum_backend"]