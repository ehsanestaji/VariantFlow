#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --release
mkdir -p tests/output

FAST_CMD="./target/release/vcf-fast filter tests/data/example.vcf --where 'QUAL > 30' -o tests/output/bench.fast.vcf"
STATS_CMD="./target/release/vcf-fast stats tests/data/example.vcf > tests/output/bench.stats.json"
DIFF_CMD="./target/release/vcf-fast diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o tests/output/bench.diff.tsv"

if command -v hyperfine >/dev/null 2>&1; then
  hyperfine "$FAST_CMD" "$STATS_CMD" "$DIFF_CMD"
else
  echo "hyperfine not found; running smoke benchmark command once"
  eval "$FAST_CMD"
  eval "$STATS_CMD"
  eval "$DIFF_CMD"
fi
