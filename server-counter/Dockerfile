FROM rust:1-bullseye

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin server_counter

FROM debian:bullseye

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=0 /tmp/compile/target/release/server_counter /srv/server_counter/server_counter
RUN chmod +x /srv/server_counter/server_counter

RUN useradd -m container
USER container
WORKDIR /srv/server_counter

CMD ["/srv/server_counter/server_counter"]