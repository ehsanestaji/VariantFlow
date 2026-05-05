#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v11-parallel-filter}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V11_PARALLEL_REPORT:-benchmark/reports/v11-parallel-native-filter-benchmark.md}"
SIZES="${VCF_FAST_V11_PARALLEL_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
THREADS="${VCF_FAST_NATIVE_FILTER_THREADS_BENCH:-4}"
BATCH_RECORDS="${VCF_FAST_NATIVE_FILTER_BATCH_RECORDS_BENCH:-4096}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.1 parallel native filter evidence" >&2
    exit 2
  fi
}

tool_version() {
  local tool="$1"
  "$tool" --version 2>&1 | head -n 1
}

markdown_cell() {
  sed 's/|/\&#124;/g' <<<"$1"
}

file_size_bytes() {
  wc -c <"$1" | tr -d ' '
}

extract_core_records() {
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$1"
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
    "$@" >/dev/null
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

default, parallel, bcftools = results
default_mean = float(default["mean"])
parallel_mean = float(parallel["mean"])
bcftools_mean = float(bcftools["mean"])
print(
    fmt(default_mean),
    fmt(default.get("stddev")),
    fmt(parallel_mean),
    fmt(parallel.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{default_mean / parallel_mean:.2f}x" if parallel_mean > 0 else "n/a",
    f"{bcftools_mean / parallel_mean:.2f}x" if parallel_mean > 0 else "n/a",
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
  echo "# VCF-Fast v1.1 Parallel Native Filter Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report tracks opt-in parallel native predicate evaluation. The implementation keeps line-preserving output by evaluating bounded batches in parallel and writing accepted original records in input order."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Dataset source: deterministic stress data from \`benchmark/generate_stress_vcf.sh\`"
  echo "- Dataset shape: stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD"
  echo "- Record tiers: \`$SIZES\`"
  echo "- Native filter threads: \`$THREADS\`"
  echo "- Native filter batch records: \`$BATCH_RECORDS\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "## Measured Parallel Filter Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact default command | exact parallel command | exact competitor command | correctness result | default mean/stddev | parallel mean/stddev | bcftools mean/stddev | parallel vs default | parallel vs bcftools | variants/sec default/parallel/bcftools | peak RSS default/parallel/bcftools | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  dataset="$DATA_DIR/stress-${records}.vcf"
  ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"

  default_out="$OUT_DIR/default-any-format-ad-${records}.vcf"
  parallel_out="$OUT_DIR/parallel-any-format-ad-${records}.vcf"
  bcftools_out="$OUT_DIR/bcftools-any-format-ad-${records}.vcf"
  default_records="$OUT_DIR/default-any-format-ad-${records}.records"
  parallel_records="$OUT_DIR/parallel-any-format-ad-${records}.records"
  bcftools_records="$OUT_DIR/bcftools-any-format-ad-${records}.records"
  hyperfine_json="$OUT_DIR/hyperfine-any-format-ad-${records}.json"

  default_command="env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $default_out"
  parallel_command="VCF_FAST_NATIVE_FILTER_THREADS=$THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $parallel_out"
  competitor_command="bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $bcftools_out"

  env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$default_out"
  VCF_FAST_NATIVE_FILTER_THREADS="$THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$parallel_out"
  bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$bcftools_out"

  diff -u "$default_out" "$parallel_out" >"$OUT_DIR/equivalence-default-parallel-${records}.diff"
  extract_core_records "$default_out" >"$default_records"
  extract_core_records "$parallel_out" >"$parallel_records"
  extract_core_records "$bcftools_out" >"$bcftools_records"
  diff -u "$bcftools_records" "$default_records" >"$OUT_DIR/equivalence-default-bcftools-${records}.diff"
  diff -u "$bcftools_records" "$parallel_records" >"$OUT_DIR/equivalence-parallel-bcftools-${records}.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/default-any-format-ad-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_FILTER_THREADS=$THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/parallel-any-format-ad-${records}.timed.vcf" \
    "bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $OUT_DIR/bcftools-any-format-ad-${records}.timed.vcf"

  read -r default_mean default_stddev parallel_mean parallel_stddev bcftools_mean bcftools_stddev parallel_vs_default parallel_vs_bcftools < <(summarize_three_way_hyperfine "$hyperfine_json")

  default_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-default-any-format-ad-${records}.txt" env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/default-any-format-ad-${records}.rss.vcf")"
  parallel_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-parallel-any-format-ad-${records}.txt" env VCF_FAST_NATIVE_FILTER_THREADS="$THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/parallel-any-format-ad-${records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-any-format-ad-${records}.txt" bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$OUT_DIR/bcftools-any-format-ad-${records}.rss.vcf")"

  default_vps="$(variants_per_second "$records" "$default_mean")"
  parallel_vps="$(variants_per_second "$records" "$parallel_mean")"
  bcftools_vps="$(variants_per_second "$records" "$bcftools_mean")"

  claim="correctness matched; inspect parallel overhead before claiming a win"
  if python3 - "${parallel_vs_default%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="parallel native measured faster than default native on this CPU-heavy expression"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | %s | %s +/- %s | %s +/- %s | %s +/- %s | %s | %s | %s / %s / %s | %s / %s / %s KB | %s | %s |\n' \
    "Stress ANY FORMAT/AD filter" \
    "deterministic stress VCF" \
    "$dataset_size" \
    "$records" \
    "$(markdown_cell "$default_command")" \
    "$(markdown_cell "$parallel_command")" \
    "$(markdown_cell "$competitor_command")" \
    "parallel native matches default native byte-for-byte and matches default native and bcftools filtered core records; line-preserving output retained" \
    "$default_mean" "$default_stddev" \
    "$parallel_mean" "$parallel_stddev" \
    "$bcftools_mean" "$bcftools_stddev" \
    "$parallel_vs_default" \
    "$parallel_vs_bcftools" \
    "$default_vps" "$parallel_vps" "$bcftools_vps" \
    "$default_rss" "$parallel_rss" "$bcftools_rss" \
    "synthetic stress CPU-heavy expression only; I/O-bound filters may not benefit" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Working datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-*.json\`"
  echo "- Equivalence diffs: \`$OUT_DIR/equivalence-*.diff\`"
  echo "- Peak RSS files: \`$OUT_DIR/rss-*.txt\`"
} >>"$REPORT"

echo "v1.1 parallel native filter report written to $REPORT"
