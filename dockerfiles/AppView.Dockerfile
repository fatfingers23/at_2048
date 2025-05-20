FROM rust:1.86.0-bookworm AS api-builder
WORKDIR /app
COPY ../ /app
RUN cargo build --bin appview_2048 --release
#
FROM rust:1.86-slim-bookworm AS api
COPY --from=api-builder /app/target/release/appview_2048 /usr/local/bin/apview_2048
COPY --from=api-builder /app/appview_2048/Dev.toml Dev.toml
CMD ["appview_2048"]