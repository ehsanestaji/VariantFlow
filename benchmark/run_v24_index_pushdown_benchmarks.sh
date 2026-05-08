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
SYNTHETIC_CHUNK_RECORDS="${VCF_FAST_V24_SYNTHETIC_CHUNK_RECORDS:-8192}"
PUBLIC_CHUNK_RECORDS="${VCF_FAST_V24_PUBLIC_CHUNK_RECORDS:-8192 1024 256}"
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
    echo "- Synthetic chunk_record_target: \`$SYNTHETIC_CHUNK_RECORDS\`"
    echo "- Public chunk_record_target matrix: \`$PUBLIC_CHUNK_RECORDS\`"
    echo "- Output directory: \`$OUT_DIR\`"
    echo
    echo "## Measured Rows"
    echo
    echo "| case | chunk_record_target | tier | predicate | used VFI | chunks scanned | chunks skipped | skip rate | fallback reason | index build cost | break-even query count | runtime mean/stddev | speedup | peak RSS KB | correctness result | exact VariantFlow command | exact competitor command | claim decision |"
    echo "| --- | ---: | ---: | --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  } >"$REPORT"
}

run_v21_case() {
  local label="$1"
  local expr="$2"
  local bcftools_expr="$3"
  local mode="${4:-synthetic}"
  local chunk_records="${5:-8192}"
  local case_dir="$OUT_DIR/$label"
  mkdir -p "$case_dir"

  VCF_FAST_INDEX_CHUNK_RECORDS="$chunk_records" \
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
    echo "- chunk_record_target: \`$chunk_records\`"
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

run_v21_case "qual-high-skip" "QUAL > 1000" "QUAL>1000" "synthetic" "$SYNTHETIC_CHUNK_RECORDS"
run_v21_case "filter-pass" "FILTER == \"PASS\"" "FILTER=\"PASS\"" "synthetic" "$SYNTHETIC_CHUNK_RECORDS"
run_v21_case "dp-high-skip" "DP > 1000" "INFO/DP>1000" "synthetic" "$SYNTHETIC_CHUNK_RECORDS"
run_v21_case "indexed-info-key" "$INDEX_INFO_KEY > 1000" "INFO/$INDEX_INFO_KEY>1000" "synthetic" "$SYNTHETIC_CHUNK_RECORDS"

run_v21_case "public-qual-high-skip" "QUAL > 1000" "QUAL>1000" "public-igsr" "8192"
run_v21_case "public-filter-pass" "FILTER == \"PASS\"" "FILTER=\"PASS\"" "public-igsr" "8192"
for chunk_records in $PUBLIC_CHUNK_RECORDS; do
  run_v21_case "public-af-high-skip-chunk-${chunk_records}" "AF > 0.99" "INFO/AF>0.99" "public-igsr" "$chunk_records"
done

echo "Wrote v2.4 index pushdown report to $REPORT"
