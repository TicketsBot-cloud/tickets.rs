FROM rust:1-bullseye AS builder

RUN apt-get update && apt-get -y upgrade && apt-get -y install git openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin vote_listener

# Prod container
FROM debian:bullseye

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=builder /tmp/compile/target/release/vote_listener /srv/vote_listener/vote_listener
RUN chmod +x /srv/vote_listener/vote_listener

RUN useradd -m container
USER container
WORKDIR /srv/vote_listener

CMD ["/srv/vote_listener/vote_listener"]