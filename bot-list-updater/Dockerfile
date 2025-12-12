FROM rust:1-bullseye AS builder

RUN apt-get update && apt-get -y upgrade && apt-get -y install git openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin bot_list_updater

# Prod container
FROM debian:bullseye

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=builder /tmp/compile/target/release/bot_list_updater /srv/bot_list_updater/bot_list_updater
RUN chmod +x /srv/bot_list_updater/bot_list_updater

RUN useradd -m container
USER container
WORKDIR /srv/bot_list_updater

CMD ["/srv/bot_list_updater/bot_list_updater"]