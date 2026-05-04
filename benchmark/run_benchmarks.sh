#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --release
mkdir -p tests/output

FAST_CMD="./target/release/vcf-fast filter tests/data/example.vcf --where 'QUAL > 30' -o tests/output/bench.fast.vcf"

if command -v hyperfine >/dev/null 2>&1; then
  hyperfine "$FAST_CMD"
else
  echo "hyperfine not found; running smoke benchmark command once"
  eval "$FAST_CMD"
fi
