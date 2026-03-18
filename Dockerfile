# 1) Build stage
FROM rust:1.88-bullseye AS builder

WORKDIR /app

# Cache dependencies properly
COPY Cargo.toml Cargo.lock ./

# Create dummy main so Cargo is happy
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will rebuild when Cargo.toml changes)
RUN cargo build --release

# Remove dummy source
RUN rm -rf src

# Copy real source
COPY src ./src

# Force rebuild of app (and re-link with updated deps/features)
RUN cargo build --release

# Copy real source and build
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# 2) Final lightweight stage
FROM debian:bookworm-slim

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/axum_backend /usr/local/bin/axum_backend

EXPOSE 3000

# Run the server
CMD ["axum_backend"]
