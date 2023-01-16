FROM debian:jessie AS builder
RUN apt-get update && apt-get upgrade -y
RUN apt-get update && apt-get install -y --force-yes curl libmysqlclient-dev build-essential pkg-config libssl-dev ca-certificates

# Install rust
RUN curl https://sh.rustup.rs/ -sSf | \
  sh -s -- -y --default-toolchain nightly

ENV PATH="/root/.cargo/bin:${PATH}"

ADD . ./

RUN cargo build --release

FROM debian:jessie
RUN apt-get update && apt-get upgrade -y
RUN apt-get update && apt-get install -y --force-yes libmysqlclient-dev pkg-config libssl-dev ca-certificates

COPY --from=builder \
  /target/release/rust-remote-pi-monitor \
  /usr/local/bin/

WORKDIR /root
CMD ROCKET_PORT=$PORT /usr/local/bin/rust-remote-pi-monitor
