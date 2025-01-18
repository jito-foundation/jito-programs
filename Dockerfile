# syntax=docker/dockerfile:1.4.0
FROM rust:1.75.0-slim-bullseye as builder

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

RUN sh -c "$(curl -sSfL https://release.solana.com/v1.18.9/install)"

ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

RUN cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked

WORKDIR /jito-programs
COPY . .
RUN mkdir -p ./container-out

RUN solana --version
RUN --mount=type=cache,mode=0777,target=/jito-programs/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
      cd ./mev-programs && anchor build && cp target/deploy/* ../container-out;

FROM debian:bullseye-slim

COPY --from=builder /jito-programs/container-out /jito-programs/container-out

WORKDIR /jito-programs
