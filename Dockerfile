# 1) Build stage
FROM rust:1.88-bullseye AS builder

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

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
