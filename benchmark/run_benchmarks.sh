#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --release
mkdir -p tests/output

DATASET="tests/output/benchmark-data/synthetic.vcf"
FILTER_OUT="tests/output/bench.fast.vcf"
STATS_OUT="tests/output/bench.stats.json"
DIFF_OUT="tests/output/bench.diff.tsv"
DIFF_SUMMARY="tests/output/bench.diff.summary.txt"

./benchmark/generate_synthetic_vcf.sh "$DATASET" "${VCF_FAST_BENCH_RECORDS:-10000}"

./target/release/vcf-fast filter "$DATASET" --where 'QUAL > 30' -o "$FILTER_OUT"
awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } $6 <= 30 { ok = 0; print "record failed QUAL > 30: " $0 > "/dev/stderr" } END { exit ok ? 0 : 1 }' "$FILTER_OUT"

./target/release/vcf-fast stats "$DATASET" >"$STATS_OUT"
grep -q "\"variants\": ${VCF_FAST_BENCH_RECORDS:-10000}" "$STATS_OUT"

./target/release/vcf-fast diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o "$DIFF_OUT" 2>"$DIFF_SUMMARY"
grep -q 'shared=1 only_in_a=2 only_in_b=1' "$DIFF_SUMMARY"

FAST_CMD="./target/release/vcf-fast filter $DATASET --where 'QUAL > 30' -o tests/output/bench.fast.timed.vcf"
STATS_CMD="./target/release/vcf-fast stats $DATASET > tests/output/bench.stats.timed.json"
DIFF_CMD="./target/release/vcf-fast diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o tests/output/bench.diff.timed.tsv"

if command -v hyperfine >/dev/null 2>&1; then
  if command -v bcftools >/dev/null 2>&1; then
    BCFTOOLS_OUT="tests/output/bench.bcftools.vcf"
    BCFTOOLS_CMD="bcftools filter -i 'QUAL>30' $DATASET -o $BCFTOOLS_OUT"
    hyperfine "$FAST_CMD" "$BCFTOOLS_CMD" "$STATS_CMD" "$DIFF_CMD"
    grep -v '^#' "$FILTER_OUT" >tests/output/bench.fast.records
    grep -v '^#' "$BCFTOOLS_OUT" >tests/output/bench.bcftools.records
    diff -u tests/output/bench.bcftools.records tests/output/bench.fast.records
  else
    echo "bcftools not found; running vcf-fast-only hyperfine benchmarks"
    hyperfine "$FAST_CMD" "$STATS_CMD" "$DIFF_CMD"
  fi
else
  echo "hyperfine not found; completed smoke correctness checks only"
fi
