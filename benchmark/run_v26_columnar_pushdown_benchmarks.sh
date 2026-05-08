#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SIZES="${VCF_FAST_V26_SIZES:-100000 1000000}"
RUNS="${VCF_FAST_V26_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V26_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
REPEATS="${VCF_FAST_V26_REPEATED_QUERIES:-${VCF_FAST_COLUMNAR_REPEATED_QUERIES:-5}}"
ROW_GROUPS="${VCF_FAST_V26_ROW_GROUP_RECORDS:-8192 65536 262144}"
OUT_DIR="${VCF_FAST_V26_OUT_DIR:-tests/output/benchmark-results/v26-columnar-pushdown}"
REPORT="${VCF_FAST_V26_REPORT:-$OUT_DIR/v26-columnar-pushdown-benchmark.md}"
# DuckDB query label retained for harness tests and report readers: FILTER == "PASS"

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

{
  echo "# VariantFlow v2.6 Columnar Pushdown Benchmark"
  echo
  echo "This report measures Parquet row-group sizing and DuckDB pushdown-oriented repeated queries for CHROM, FILTER, QUAL, INFO/DP, and INFO/AF. The goal is to report export time, query-only time, amortized time, RSS, and break-even query count against repeated VCF scans."
  echo
  echo "## Configuration"
  echo
  echo "- Record tiers: \`$SIZES\`"
  echo "- Runs: \`$RUNS\`"
  echo "- Warmup: \`$WARMUP\`"
  echo "- Repeated DuckDB queries: \`$REPEATS\`"
  echo "- Row-group candidates via \`VCF_FAST_PARQUET_ROW_GROUP_RECORDS\`: \`$ROW_GROUPS\`"
  echo
  echo "## Queries"
  echo
  echo "- \`QUAL > 30\`"
  echo "- \`INFO/DP > 40\`"
  echo "- \`FILTER == \"PASS\"\`"
  echo "- \`GROUP BY CHROM, FILTER\`"
  echo
  echo "## Measured Rows"
  echo
  echo "| dataset | tier | row-group sizing | query | export time | query-only time | amortized time | break-even query count | peak RSS KB | exact export command | exact DuckDB command | exact competitor command | correctness result | caveat |"
  echo "| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  echo "| pending | pending | pending | DuckDB | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | claim updates wait for matched baselines |"
} >"$REPORT"

if [[ "${VCF_FAST_V26_DRY_RUN:-0}" == "1" ]]; then
  echo "Dry run: wrote $REPORT"
  exit 0
fi

for row_group in $ROW_GROUPS; do
  VCF_FAST_PARQUET_ROW_GROUP_RECORDS="$row_group" \
  VCF_FAST_V10_COLUMNAR_SIZES="$SIZES" \
  VCF_FAST_BENCH_RUNS="$RUNS" \
  VCF_FAST_BENCH_WARMUP="$WARMUP" \
  VCF_FAST_COLUMNAR_REPEATED_QUERIES="$REPEATS" \
  VCF_FAST_BENCH_OUT_DIR="$OUT_DIR/row-group-$row_group" \
  VCF_FAST_V10_COLUMNAR_REPORT="$OUT_DIR/row-group-$row_group/report.md" \
  ./benchmark/run_v10_columnar_workflow_benchmarks.sh

  {
    echo
    echo "### Row group $row_group"
    echo
    echo "- Source report: \`$OUT_DIR/row-group-$row_group/report.md\`"
    echo "- Metadata columns inspected by DuckDB: CHROM, FILTER, QUAL, INFO/DP, INFO/AF."
    echo "- Rows should become claims only when DuckDB query output matches normalized VCF or bcftools baselines."
  } >>"$REPORT"
done

echo "Wrote v2.6 columnar pushdown report to $REPORT"
