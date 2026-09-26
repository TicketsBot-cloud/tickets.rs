FROM rust:1-bookworm

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

# global-resolver (a dependency of secure-proxy) requires nightly-only language
# features (#![feature(ip)], #![feature(impl_trait_in_assoc_type)]), matching
# the nightly toolchain already used for it in .github/workflows/check.yaml.
RUN rustup install nightly && rustup default nightly
RUN cargo build --release --bin secure-proxy

# Prod container
FROM debian:bookworm

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=0 /tmp/compile/target/release/secure-proxy /srv/proxy/secure-proxy
RUN chmod +x /srv/proxy/secure-proxy

RUN useradd -m container
USER container
WORKDIR /srv/proxy

CMD ["/srv/proxy/secure-proxy"]
