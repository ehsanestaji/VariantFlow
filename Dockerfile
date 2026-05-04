FROM rust:1.95-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends bcftools ca-certificates clang cmake curl gzip hyperfine libclang-dev make pkg-config python3 tabix time zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

WORKDIR /work

CMD ["cargo", "test"]
