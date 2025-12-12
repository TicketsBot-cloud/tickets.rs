FROM rust:1-bullseye AS builder

RUN apt-get update && apt-get -y upgrade && apt-get -y install git openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin cache

# Prod container
FROM debian:bullseye

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=builder /tmp/compile/target/release/cache /srv/cache/cache
RUN chmod +x /srv/cache/cache

RUN useradd -m container
USER container
WORKDIR /srv/cache

CMD ["/srv/cache/cache"]