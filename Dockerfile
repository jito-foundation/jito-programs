# syntax=docker/dockerfile:1.4.0
FROM rust:1.66.0-slim-bullseye as builder

RUN set -x \
    && apt-get -qq update \
    && apt-get -qq -y install \
    clang \
    cmake \
    libudev-dev \
    unzip \
    libssl-dev \
    pkg-config \
    zlib1g-dev \
    curl

RUN sh -c "$(curl -sSfL https://release.solana.com/v1.13.5/install)"

ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

RUN cargo install --git https://github.com/coral-xyz/anchor --tag v0.26.0 anchor-cli --locked

WORKDIR /jito-programs
COPY . .
RUN mkdir -p ./container-out

# Uses docker buildkite to cache the image.
RUN solana --version
RUN --mount=type=cache,mode=0777,target=/jito-programs/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
      cd ./mev-programs && anchor build && cp target/deploy/* ../container-out;