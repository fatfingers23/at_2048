##Builds the assets for the WASM app
#FROM rust:1.86.0-bookworm AS wasm-builder
#WORKDIR /app
#COPY ../ /app
#RUN rustup target add wasm32-unknown-unknown
#RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
#RUN cargo binstall trunk
#RUN cargo binstall wasm-bindgen-cli
#WORKDIR /app/client_2048
#RUN trunk build --release
#


FROM caddy:2.1.0-alpine AS caddy
EXPOSE 80
EXPOSE 443
EXPOSE 443/udp
COPY ../production_configs/Caddyfile /etc/caddy/Caddyfile
COPY ../client_2048/dist /srv
#COPY --from=wasm-builder /app/client_2048/dist /srv
COPY ../production_configs/client_metadata.json /srv/client_metadata.json