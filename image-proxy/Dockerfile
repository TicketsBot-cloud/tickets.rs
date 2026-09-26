FROM rust:1-bookworm

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

# global-resolver needs nightly-only features
RUN rustup install nightly && rustup default nightly
RUN cargo build --release --bin image-proxy

# Prod container
FROM debian:bookworm

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=0 /tmp/compile/target/release/image-proxy /srv/proxy/image-proxy
RUN chmod +x /srv/proxy/image-proxy

RUN useradd -m container
USER container
WORKDIR /srv/proxy

CMD ["/srv/proxy/image-proxy"]
