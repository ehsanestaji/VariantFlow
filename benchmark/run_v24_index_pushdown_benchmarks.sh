#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SIZES="${VCF_FAST_V24_SIZES:-100000 1000000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
OUT_DIR="${VCF_FAST_V24_OUT_DIR:-tests/output/benchmark-results/v24-index-pushdown}"
REPORT="${VCF_FAST_V24_REPORT:-$OUT_DIR/v24-index-pushdown-benchmark.md}"
INDEX_INFO_KEY="${VCF_FAST_V24_INDEX_INFO_KEY:-DP}"
# Planner case label retained for harness tests and report readers: FILTER == "PASS"

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

write_header() {
  {
    echo "# VariantFlow v2.4 Index Pushdown Benchmark"
    echo
    echo "This report measures guarded .vfi pushdown for CHROM, POS, QUAL, FILTER, INFO/DP, INFO/AF, and indexed INFO/<KEY> predicates such as FILTER == \"PASS\". Chunks are skipped only when metadata proves no record can pass; otherwise VariantFlow must fall back to normal streaming."
    echo
    echo "## Configuration"
    echo
    echo "- Record tiers: \`$SIZES\`"
    echo "- Runs: \`$RUNS\`"
    echo "- Warmup: \`$WARMUP\`"
    echo "- Indexed INFO/<KEY>: \`INFO/$INDEX_INFO_KEY\`"
    echo "- Output directory: \`$OUT_DIR\`"
    echo
    echo "## Measured Rows"
    echo
    echo "| case | tier | predicate | used VFI | chunks scanned | chunks skipped | skip rate | fallback reason | index build cost | break-even query count | runtime mean/stddev | speedup | peak RSS KB | correctness result | exact VariantFlow command | exact competitor command | claim decision |"
    echo "| --- | ---: | --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  } >"$REPORT"
}

run_v21_case() {
  local label="$1"
  local expr="$2"
  local bcftools_expr="$3"
  local mode="${4:-synthetic}"
  local case_dir="$OUT_DIR/$label"
  mkdir -p "$case_dir"

  VCF_FAST_V21_MODE="$mode" \
  VCF_FAST_V21_SIZES="$SIZES" \
  VCF_FAST_V21_EXPR="$expr" \
  VCF_FAST_V21_BCFTOOLS_EXPR="$bcftools_expr" \
  VCF_FAST_BENCH_RUNS="$RUNS" \
  VCF_FAST_BENCH_WARMUP="$WARMUP" \
  VCF_FAST_V21_OUT_DIR="$case_dir" \
  VCF_FAST_V21_REPORT="$case_dir/report.md" \
  ./benchmark/run_v21_indexed_filter_benchmarks.sh

  {
    echo
    echo "### $label"
    echo
    echo "- Source report: \`$case_dir/report.md\`"
    echo "- Planner fields expected in source report: chunks_total, chunks_skipped, records_skipped_estimate, fallback reason, index build cost, break-even query count."
    echo "- Correctness baseline: \`bcftools filter\` core records."
    echo "- Note: rows are accepted into public claims only when correctness result matched and the harness reports either used VFI or fell back safely."
  } >>"$REPORT"
}

write_header

if [[ "${VCF_FAST_V24_DRY_RUN:-0}" == "1" ]]; then
  echo "Dry run: benchmark orchestration written to $REPORT"
  exit 0
fi

run_v21_case "qual-high-skip" "QUAL > 1000" "QUAL>1000" "synthetic"
run_v21_case "filter-pass" "FILTER == \"PASS\"" "FILTER=\"PASS\"" "synthetic"
run_v21_case "af-high-skip" "AF > 0.99" "INFO/AF>0.99" "public-igsr"
run_v21_case "dp-high-skip" "DP > 1000" "INFO/DP>1000" "synthetic"
run_v21_case "indexed-info-key" "$INDEX_INFO_KEY > 1000" "INFO/$INDEX_INFO_KEY>1000" "synthetic"

echo "Wrote v2.4 index pushdown report to $REPORT"
