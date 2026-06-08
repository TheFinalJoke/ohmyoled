# Prod build for ohmyoled. Multi-stage: rust toolchain in the build
# stage, slim debian for the runtime image so we don't ship cargo + rustc.
#
# The project is pure Rust since v3.0.0 — no Python, no rpi-rgb-led-matrix
# C library, no pip install. reqwest uses rustls-tls so OpenSSL headers
# aren't required either. Fonts are not tracked in git; they're fetched
# at build time via scripts/fetch-fonts.sh from the GitHub Release.

ARG RUST_VERSION="1.83"
ARG OHMYOLED_REF="dev"

FROM rust:${RUST_VERSION}-bullseye AS builder

# git for the clone, curl for fetch-fonts.sh, ca-certificates so HTTPS works.
RUN apt-get update && apt-get install -y --no-install-recommends \
        git \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
ARG OHMYOLED_REF
RUN git clone --branch "${OHMYOLED_REF}" --depth 1 \
        https://github.com/TheFinalJoke/ohmyoled.git .

RUN bash scripts/fetch-fonts.sh
# `--features eink` adds the Waveshare SPI backend alongside the default
# `hardware` (LED panel) backend. Both deps are arm/aarch64-gated in
# ohmyoled-matrix/Cargo.toml, so this is a no-op on x86_64 source builds and
# enables e-ink panels when this image is built on a Pi target.
RUN cargo build --release --features eink

# ---------------------------------------------------------------------------
# Runtime stage — just the binary + fonts + a couple of system fonts the
# matrix never uses but that the apt package is tiny and other tools expect.
# ---------------------------------------------------------------------------
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        fonts-noto-mono \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /var/ohmyoled /etc/ohmyoled

COPY --from=builder /build/target/release/ohmyoled /usr/local/bin/ohmyoled
COPY --from=builder /build/fonts/ /usr/share/fonts/

ENV RUST_BACKTRACE=full
ENTRYPOINT ["/usr/local/bin/ohmyoled"]
