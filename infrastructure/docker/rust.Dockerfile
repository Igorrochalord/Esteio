# Toolchain de desenvolvimento do Núcleo ERP (Rust + dependências do Tauri no Linux + Node 22).
# Usada pelo Makefile para test/lint/build sem instalar nada no host.
FROM node:22-bookworm-slim AS node

FROM rust:1.91-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
        libssl-dev libxdo-dev file \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt

COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s /usr/local/lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s /usr/local/lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx

# CARGO_HOME num volume nomeado (cache de crates); gravável pelo uid do host.
ENV CARGO_HOME=/cargo \
    npm_config_cache=/tmp/.npm \
    HOME=/tmp
RUN mkdir -p /cargo && chmod 777 /cargo

WORKDIR /work
