FROM rust:1.95-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends bcftools ca-certificates curl gzip hyperfine make python3 tabix time \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

WORKDIR /work

CMD ["cargo", "test"]
