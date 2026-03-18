# 1) Build stage
FROM rust:1.88-bullseye AS builder

WORKDIR /app

COPY . .

RUN cargo clean
RUN cargo build --release


# 2) Runtime stage (MATCH bullseye)
FROM debian:bullseye-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/axum_backend /usr/local/bin/axum_backend

EXPOSE 3000

CMD ["axum_backend"]