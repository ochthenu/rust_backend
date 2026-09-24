# 1) Build stage
FROM rust:1.94-bookworm AS builder

WORKDIR /app

COPY . .

RUN cargo clean
RUN cargo build --release


# 2) Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/axum_backend /usr/local/bin/axum_backend

EXPOSE 3000

CMD ["axum_backend"]