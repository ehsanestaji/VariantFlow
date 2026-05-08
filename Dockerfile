FROM rust:1.95-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends bcftools ca-certificates clang cmake curl gzip hyperfine libclang-dev make pkg-config python3 python3-venv tabix time vcftools zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

RUN python3 -m venv /opt/duckdb-venv \
    && /opt/duckdb-venv/bin/pip install --no-cache-dir duckdb==1.5.2

ENV VCF_FAST_PYTHON=/opt/duckdb-venv/bin/python

RUN rustup component add rustfmt clippy

WORKDIR /work

CMD ["cargo", "test"]
