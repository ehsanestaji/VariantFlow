#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v10-compressed}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V10_REPORT:-benchmark/reports/v10-compressed-input-benchmark.md}"
SIZES="${VCF_FAST_V10_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
THREADS="${VCF_FAST_NATIVE_BGZF_THREADS_BENCH:-4}"
REGION="${VCF_FAST_HEAVY_REGION:-chr22:1-20000000}"
SOURCE="${VCF_FAST_V10_SOURCE:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.0 compressed-input evidence" >&2
    exit 2
  fi
}

tool_version() {
  local tool="$1"
  "$tool" --version 2>&1 | head -n 1
}

markdown_cell() {
  local value="$1"
  printf '%s\n' "$value" | sed 's/|/\&#124;/g'
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
    "$@"
    echo "n/a"
  fi
}

stage_bgzf_dataset() {
  local output="$1"
  local records="$2"

  rm -f "$output" "${output}.tbi"
  {
    bcftools view -h "$SOURCE"
    (
      set +o pipefail
      bcftools view -H -r "$REGION" "$SOURCE" \
        | awk -v limit="$records" '
            NR <= limit { print; count++ }
            NR == limit { exit }
            END { if ((count + 0) == 0) exit 77 }
          '
    )
  } | bgzip -c >"$output"
  tabix -f -p vcf "$output"
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

default, threaded, bcftools = results
default_mean = float(default["mean"])
threaded_mean = float(threaded["mean"])
bcftools_mean = float(bcftools["mean"])

print(
    fmt(default_mean),
    fmt(default.get("stddev")),
    fmt(threaded_mean),
    fmt(threaded.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{default_mean / threaded_mean:.2f}x" if threaded_mean > 0 else "n/a",
    f"{bcftools_mean / threaded_mean:.2f}x" if threaded_mean > 0 else "n/a",
)
PY
}

if [[ ! -s "$SOURCE" ]]; then
  echo "missing $SOURCE; run benchmark/download_public_data.sh igsr-chr22 first" >&2
  exit 2
fi

require_tool bcftools
require_tool bgzip
require_tool tabix
require_tool hyperfine

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi

cargo build --release

{
  echo "# VCF-Fast v1.0 Compressed Input Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report tracks the first v1.0 parallel-compressed-input slice. Runtime claims are limited to rows where threaded native BGZF output matches the \`bcftools filter\` core-record baseline."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Dataset source: $SOURCE"
  echo "- Region: \`$REGION\`"
  echo "- Record tiers: \`$SIZES\`"
  echo "- Native BGZF threads: \`$THREADS\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "## Measured Compressed Input Cases"
  echo
  echo "| case | dataset size bytes | record count | exact default VCF-Fast command | exact threaded VCF-Fast command | exact competitor command | correctness result | default mean/stddev | threaded mean/stddev | bcftools mean/stddev | threaded vs default | threaded vs bcftools | variants/sec default/threaded/bcftools | peak RSS default/threaded/bcftools | caveat | claim decision |"
  echo "| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
  stage_bgzf_dataset "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"

  default_out="$OUT_DIR/default-qual-${records}.vcf"
  threaded_out="$OUT_DIR/threaded-qual-${records}.vcf"
  bcftools_out="$OUT_DIR/bcftools-qual-${records}.vcf"
  default_records="$OUT_DIR/default-qual-${records}.records"
  threaded_records="$OUT_DIR/threaded-qual-${records}.records"
  bcftools_records="$OUT_DIR/bcftools-qual-${records}.records"
  hyperfine_json="$OUT_DIR/hyperfine-qual-${records}.json"
  diff_threaded="$OUT_DIR/equivalence-threaded-qual-${records}.diff"
  diff_default="$OUT_DIR/equivalence-default-qual-${records}.diff"

  default_command="env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $default_out"
  threaded_command="VCF_FAST_NATIVE_BGZF_THREADS=$THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $threaded_out"
  competitor_command="bcftools filter -i 'QUAL>30' $dataset -o $bcftools_out"

  env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$default_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$THREADS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$threaded_out"
  bcftools filter -i "QUAL>30" "$dataset" -o "$bcftools_out"

  extract_core_records "$default_out" >"$default_records"
  extract_core_records "$threaded_out" >"$threaded_records"
  extract_core_records "$bcftools_out" >"$bcftools_records"
  diff -u "$bcftools_records" "$threaded_records" >"$diff_threaded"
  diff -u "$bcftools_records" "$default_records" >"$diff_default"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/default-qual-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/threaded-qual-${records}.timed.vcf" \
    "bcftools filter -i 'QUAL>30' $dataset -o $OUT_DIR/bcftools-qual-${records}.timed.vcf"

  read -r default_mean default_stddev threaded_mean threaded_stddev bcftools_mean bcftools_stddev threaded_vs_default threaded_vs_bcftools < <(summarize_three_way_hyperfine "$hyperfine_json")

  default_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-default-qual-${records}.txt" env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/default-qual-${records}.rss.vcf")"
  threaded_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-threaded-qual-${records}.txt" env VCF_FAST_NATIVE_BGZF_THREADS="$THREADS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/threaded-qual-${records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-qual-${records}.txt" bcftools filter -i "QUAL>30" "$dataset" -o "$OUT_DIR/bcftools-qual-${records}.rss.vcf")"

  default_vps="$(variants_per_second "$records" "$default_mean")"
  threaded_vps="$(variants_per_second "$records" "$threaded_mean")"
  bcftools_vps="$(variants_per_second "$records" "$bcftools_mean")"

  claim="correctness matched; inspect measured speedup before claiming a win"
  if python3 - "${threaded_vs_bcftools%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="measured faster than bcftools on this BGZF input case"
  fi

  printf '| %s | %s | %s | %s | %s | %s | %s | %s +/- %s | %s +/- %s | %s +/- %s | %s | %s | %s / %s / %s | %s / %s / %s KB | %s | %s |\n' \
    "IGSR bounded BGZF QUAL filter" \
    "$dataset_size" \
    "$records" \
    "$(markdown_cell "\`$default_command\`")" \
    "$(markdown_cell "\`$threaded_command\`")" \
    "$(markdown_cell "\`$competitor_command\`")" \
    "default and threaded VCF-Fast match bcftools filtered core records" \
    "$default_mean" "$default_stddev" \
    "$threaded_mean" "$threaded_stddev" \
    "$bcftools_mean" "$bcftools_stddev" \
    "$threaded_vs_default" \
    "$threaded_vs_bcftools" \
    "$default_vps" "$threaded_vps" "$bcftools_vps" \
    "$default_rss" "$threaded_rss" "$bcftools_rss" \
    "bounded chr22 BGZF subset; ordinary gzip is still single-thread fallback" \
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

echo "v1.0 compressed-input evidence report written to $REPORT"
