FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/

RUN apt update && apt install -y iputils-ping && ping db && sleep 5

# Cache dependencies -> https://stackoverflow.com/a/58474618/4674154
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "" >src/lib.rs && \
    cargo build --release
# now remove cached lib, otherwise the stale version will be reused later
RUN cargo clean -p valetudo-dreameadapter-map --release

COPY . .
RUN cargo install --offline --path . --bins


FROM debian:bookworm-slim

COPY --from=builder /usr/local/cargo/bin/valetudo-dreameadapter-map /usr/local/bin/
CMD ["/bin/bash", "-c", "/usr/local/bin/valetudo-dreameadapter-map"]