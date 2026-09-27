# ---- Build ----
FROM rust:1.93-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
# Cache dependency compilation; the lockfile keeps it reproducible.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && cp target/release/backend /backend

# ---- Runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /backend /usr/local/bin/backend
# sqlx uses rustls (pure Rust) — no openssl needed at runtime, even for
# postgres:// URLs. SQLite needs no system libs either.
WORKDIR /data
# Vercel injects PORT (default 80 there); keep 3000 for local parity.
ENV PORT=3000
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD curl -f "http://127.0.0.1:${PORT:-3000}/api/health" || exit 1
CMD ["backend"]
