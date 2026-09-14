FROM rust:1.85-bookworm AS build
WORKDIR /app
COPY Cargo.toml ./
COPY backend-rust ./backend-rust
RUN cargo build --release -p ai-forex-backend
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/ai-forex-backend /usr/local/bin/backend
EXPOSE 8080
CMD ["backend"]

