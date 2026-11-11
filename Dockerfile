# Railway build: Dioxus fullstack bundle (server binary + web assets)
FROM rust:1-trixie AS build
RUN rustup target add wasm32-unknown-unknown \
 && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
 && cargo binstall -y dioxus-cli@0.7.10
WORKDIR /app
COPY . .
RUN dx bundle --platform web --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/dx/shorturl/release/web/ /app/
ENV IP=0.0.0.0 PORT=8080
EXPOSE 8080
CMD ["/app/server"]
