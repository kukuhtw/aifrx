FROM rust:1.88-bookworm AS build
WORKDIR /app
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY backend-rust ./backend-rust
RUN cargo build --release --locked -p ai-forex-backend
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/ai-forex-backend /usr/local/bin/backend
COPY --from=build /app/target/release/create-admin /usr/local/bin/create-admin
EXPOSE 8080
CMD ["backend"]
