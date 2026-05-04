FROM rust:1.95-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates gzip make time \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /work

CMD ["cargo", "test"]
