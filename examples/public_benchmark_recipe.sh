#!/usr/bin/env bash
set -euo pipefail

# Reproduce the public evidence rows used by the release-facing docs.
# Large artifacts stay under ignored tests/output paths.

benchmark/download_public_data.sh all

VCF_FAST_BENCH_RUNS="${VCF_FAST_BENCH_RUNS:-3}" \
VCF_FAST_BENCH_WARMUP="${VCF_FAST_BENCH_WARMUP:-1}" \
make bench-v14

VCF_FAST_BENCH_RUNS="${VCF_FAST_BENCH_RUNS:-3}" \
VCF_FAST_BENCH_WARMUP="${VCF_FAST_BENCH_WARMUP:-1}" \
make bench-v12
