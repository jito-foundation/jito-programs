# syntax=docker/dockerfile:1.4.0
FROM rust:1.66.0-slim-bullseye as builder

RUN set -x
RUN apt-get update
RUN apt-get -qq -y install \
    clang \
    cmake \
    libudev-dev \
    unzip \
    libssl-dev \
    pkg-config \
    zlib1g-dev \
    curl

RUN curl -o solana-install-init https://github.com/solana-labs/solana/releases/download/v1.14.13/solana-install-init-x86_64-unknown-linux-gnu \
    && chmod +x solana-install-init \
    && ./solana-install-init 1.14.13
# RUN cat solana-install-init
# RUN ./solana-install-init --help
# RUN kk./solana-install-init 1.14.13
ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

RUN solana --version

RUN cargo install --git https://github.com/coral-xyz/anchor --tag v0.26.0 anchor-cli --locked

WORKDIR /jito-programs
COPY . .
RUN mkdir -p ./container-out

# Uses docker buildkite to cache the image.
RUN --mount=type=cache,mode=0777,target=/jito-programs/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
      cd ./mev-programs && anchor build && cp target/deploy/* ../container-out;