FROM rust:1.93.1-bookworm AS build

# Setup env
RUN <<EOF
    apt-get update
    apt-get install -y --no-install-recommends \
        libasound2-dev \
        libudev-dev \
        libwayland-dev \
        libxkbcommon-dev
    rustup target add wasm32-unknown-unknown
    cargo install -f wasm-bindgen-cli --version 0.2.114
EOF
ADD . /project/
# Put all statis files
COPY /assets/ /dist/assets
COPY /static/* /dist
# Put built files
RUN <<EOF
    cd /project &&\
    cargo build --target wasm32-unknown-unknown --features dbg --profile wasm-release --locked &&\
    wasm-bindgen --target web --out-dir /dist ./target/wasm32-unknown-unknown/wasm-release/trishoot.wasm
EOF

FROM httpd:trixie 
COPY --from=build /dist /usr/local/apache2/htdocs/ 
