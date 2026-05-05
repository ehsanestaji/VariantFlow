#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v10-parquet}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V10_PARQUET_REPORT:-benchmark/reports/v10-parquet-export-benchmark.md}"
SIZES="${VCF_FAST_V10_PARQUET_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.0 Parquet export evidence" >&2
    exit 2
  fi
}

tool_version() {
  local tool="$1"
  "$tool" --version 2>&1 | head -n 1
}

file_size_bytes() {
  wc -c <"$1" | tr -d ' '
}

variants_per_second() {
  local records="$1"
  local mean="$2"
  python3 - "$records" "$mean" <<'PY'
import sys

records = int(sys.argv[1])
mean = sys.argv[2]
if mean == "n/a" or not mean.endswith("s"):
    print("n/a")
    raise SystemExit
seconds = float(mean[:-1])
print("n/a" if seconds <= 0 else f"{records / seconds:.0f}")
PY
}

measure_peak_rss_kb() {
  local raw_output="$1"
  shift

  if [[ "${VCF_FAST_BENCH_GNU_TIME:-0}" == "1" ]]; then
    /usr/bin/time -v -o "$raw_output" "$@"
    awk -F ':' '/Maximum resident set size/ { gsub(/ /, "", $2); print $2 }' "$raw_output"
  else
    "$@"
    echo "n/a"
  fi
}

summarize_three_way_hyperfine() {
  local json="$1"
  python3 - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def fmt(value):
    return f"{float(value or 0):.6f}s"

parquet, tsv, bcftools = results
parquet_mean = float(parquet["mean"])
tsv_mean = float(tsv["mean"])
bcftools_mean = float(bcftools["mean"])
print(
    fmt(parquet_mean),
    fmt(parquet.get("stddev")),
    fmt(tsv_mean),
    fmt(tsv.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{tsv_mean / parquet_mean:.2f}x" if parquet_mean > 0 else "n/a",
    f"{bcftools_mean / parquet_mean:.2f}x" if parquet_mean > 0 else "n/a",
)
PY
}

require_tool bcftools
require_tool hyperfine

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi

cargo build --release

{
  echo "# VCF-Fast v1.0 Parquet Export Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report tracks the first v1.0 native Parquet export slice. Correctness is covered by integration tests that read the produced Parquet through Arrow and verify schema, row count, nulls, and preserved AF strings. Runtime rows compare native Parquet export against native TSV export and \`bcftools query\` TSV projection; they are not broad Parquet workflow claims yet."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Dataset source: deterministic stress data from \`benchmark/generate_stress_vcf.sh\`"
  echo "- Dataset shape: stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD"
  echo "- Record tiers: \`$SIZES\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "## Measured Export Cases"
  echo
  echo "| case | dataset size bytes | record count | exact Parquet command | exact TSV command | exact competitor command | correctness result | parquet mean/stddev | tsv mean/stddev | bcftools mean/stddev | TSV/parquet ratio | bcftools/parquet ratio | variants/sec parquet/tsv/bcftools | peak RSS parquet/tsv/bcftools | caveat | claim decision |"
  echo "| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  dataset="$DATA_DIR/stress-${records}.vcf"
  ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"

  parquet_out="$OUT_DIR/fast-convert-parquet-${records}.parquet"
  tsv_out="$OUT_DIR/fast-convert-tsv-${records}.tsv"
  bcftools_out="$OUT_DIR/bcftools-query-tsv-${records}.tsv"
  hyperfine_json="$OUT_DIR/hyperfine-convert-${records}.json"

  parquet_command="./target/release/vcf-fast convert $dataset --to parquet -o $parquet_out"
  tsv_command="./target/release/vcf-fast convert $dataset --to tsv -o $tsv_out"
  competitor_command="bcftools query -u -f '%CHROM\\t%POS\\t%ID\\t%REF\\t%ALT\\t%QUAL\\t%FILTER\\t%INFO/DP\\t%INFO/AF\\n' $dataset > $bcftools_out"

  ./target/release/vcf-fast convert "$dataset" --to parquet -o "$parquet_out"
  ./target/release/vcf-fast convert "$dataset" --to tsv -o "$tsv_out"
  bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' "$dataset" >"$bcftools_out"

  tsv_rows="$(($(wc -l <"$tsv_out" | tr -d ' ') - 1))"
  bcftools_rows="$(wc -l <"$bcftools_out" | tr -d ' ')"
  if [[ "$tsv_rows" != "$records" || "$bcftools_rows" != "$records" ]]; then
    echo "row count mismatch for $records records" >&2
    exit 1
  fi

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "./target/release/vcf-fast convert $dataset --to parquet -o $OUT_DIR/fast-convert-parquet-${records}.timed.parquet" \
    "./target/release/vcf-fast convert $dataset --to tsv -o $OUT_DIR/fast-convert-tsv-${records}.timed.tsv" \
    "bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' $dataset > $OUT_DIR/bcftools-query-tsv-${records}.timed.tsv"

  read -r parquet_mean parquet_stddev tsv_mean tsv_stddev bcftools_mean bcftools_stddev tsv_ratio bcftools_ratio < <(summarize_three_way_hyperfine "$hyperfine_json")

  parquet_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-parquet-${records}.txt" ./target/release/vcf-fast convert "$dataset" --to parquet -o "$OUT_DIR/fast-convert-parquet-${records}.rss.parquet")"
  tsv_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-tsv-${records}.txt" ./target/release/vcf-fast convert "$dataset" --to tsv -o "$OUT_DIR/fast-convert-tsv-${records}.rss.tsv")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${records}.txt" bash -c 'bcftools query -u -f "%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n" "$1" > "$2"' _ "$dataset" "$OUT_DIR/bcftools-query-tsv-${records}.rss.tsv")"

  parquet_vps="$(variants_per_second "$records" "$parquet_mean")"
  tsv_vps="$(variants_per_second "$records" "$tsv_mean")"
  bcftools_vps="$(variants_per_second "$records" "$bcftools_mean")"

  claim="columnar output works; optimize before claiming a speed win"
  if python3 - "${bcftools_ratio%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="measured faster than bcftools query on this projection"
  fi

  printf '| %s | %s | %s | `%s` | `%s` | `%s` | %s | %s +/- %s | %s +/- %s | %s +/- %s | %s | %s | %s / %s / %s | %s / %s / %s KB | %s | %s |\n' \
    "Stress VCF selected-column export" \
    "$dataset_size" \
    "$records" \
    "$parquet_command" \
    "$tsv_command" \
    "$competitor_command" \
    "Parquet schema/null semantics verified by integration tests; TSV and bcftools row counts match input records" \
    "$parquet_mean" "$parquet_stddev" \
    "$tsv_mean" "$tsv_stddev" \
    "$bcftools_mean" "$bcftools_stddev" \
    "$tsv_ratio" \
    "$bcftools_ratio" \
    "$parquet_vps" "$tsv_vps" "$bcftools_vps" \
    "$parquet_rss" "$tsv_rss" "$bcftools_rss" \
    "synthetic stress only; no DuckDB/Polars workflow benchmark yet" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Working datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-*.json\`"
  echo "- Peak RSS files: \`$OUT_DIR/rss-*.txt\`"
} >>"$REPORT"

echo "v1.0 Parquet export report written to $REPORT"
