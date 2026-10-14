ARG PACKAGE
ARG BINARY

FROM rust:1-bookworm AS build
ARG PACKAGE
WORKDIR /src
COPY . .
RUN cargo build --locked --release --package "$PACKAGE"

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/*
ARG BINARY
COPY --from=build /src/target/release/${BINARY} /usr/local/bin/service
ENTRYPOINT ["/usr/local/bin/service"]
