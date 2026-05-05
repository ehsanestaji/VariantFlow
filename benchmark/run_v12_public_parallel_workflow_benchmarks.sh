#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v12-public-parallel-workflow}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V12_REPORT:-benchmark/reports/v12-public-parallel-workflow-benchmark.md}"
PUBLIC_TIERS="${VCF_FAST_V12_PUBLIC_TIERS:-10000 100000 1000000}"
STRESS_TIERS="${VCF_FAST_V12_STRESS_TIERS:-100000 1000000}"
REGION="${VCF_FAST_V12_REGION:-chr22:1-20000000}"
IGSR_SOURCE="${VCF_FAST_IGSR_SOURCE:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
BGZF_THREADS="${VCF_FAST_NATIVE_BGZF_THREADS_BENCH:-4}"
FILTER_THREADS="${VCF_FAST_NATIVE_FILTER_THREADS_BENCH:-4}"
BATCH_RECORDS="${VCF_FAST_NATIVE_FILTER_BATCH_RECORDS_BENCH:-4096}"
REPEATED_QUERIES="${VCF_FAST_COLUMNAR_REPEATED_QUERIES:-5}"
if [[ -z "${VCF_FAST_PYTHON:-}" && -x tests/output/benchmark-results/duckdb-venv/bin/python ]]; then
  PYTHON="tests/output/benchmark-results/duckdb-venv/bin/python"
else
  PYTHON="${VCF_FAST_PYTHON:-python3}"
fi

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.2 public parallel workflow evidence" >&2
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

count_vcf_records() {
  if [[ "$1" == *.gz ]]; then
    bcftools view -H "$1" | wc -l | tr -d ' '
  else
    awk 'BEGIN { n = 0 } !/^#/ { n++ } END { print n }' "$1"
  fi
}

extract_core_records() {
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$1"
}

variants_per_second() {
  local records="$1"
  local mean="$2"
  "$PYTHON" - "$records" "$mean" <<'PY'
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

summarize_public_filter_hyperfine() {
  local json="$1"
  "$PYTHON" - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def fmt(value):
    return f"{float(value or 0):.6f}s"

default, parallel, threaded, combined, bcftools = results
default_mean = float(default["mean"])
parallel_mean = float(parallel["mean"])
threaded_mean = float(threaded["mean"])
combined_mean = float(combined["mean"])
bcftools_mean = float(bcftools["mean"])
print(
    fmt(default_mean),
    fmt(default.get("stddev")),
    fmt(parallel_mean),
    fmt(parallel.get("stddev")),
    fmt(threaded_mean),
    fmt(threaded.get("stddev")),
    fmt(combined_mean),
    fmt(combined.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{default_mean / parallel_mean:.2f}x" if parallel_mean > 0 else "n/a",
    f"{default_mean / threaded_mean:.2f}x" if threaded_mean > 0 else "n/a",
    f"{default_mean / combined_mean:.2f}x" if combined_mean > 0 else "n/a",
    f"{bcftools_mean / threaded_mean:.2f}x" if threaded_mean > 0 else "n/a",
    f"{bcftools_mean / combined_mean:.2f}x" if combined_mean > 0 else "n/a",
)
PY
}

summarize_three_way_hyperfine() {
  local json="$1"
  "$PYTHON" - "$json" <<'PY'
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

summarize_columnar_hyperfine() {
  local json="$1"
  "$PYTHON" - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def fmt(value):
    return f"{float(value or 0):.6f}s"

export, duckdb, bcftools = results
export_mean = float(export["mean"])
duckdb_mean = float(duckdb["mean"])
bcftools_mean = float(bcftools["mean"])
workflow_mean = export_mean + duckdb_mean
print(
    fmt(export_mean),
    fmt(export.get("stddev")),
    fmt(duckdb_mean),
    fmt(duckdb.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{bcftools_mean / duckdb_mean:.2f}x" if duckdb_mean > 0 else "n/a",
    f"{bcftools_mean / workflow_mean:.2f}x" if workflow_mean > 0 else "n/a",
)
PY
}

resolve_public_region() {
  local requested="$1"
  local source="$2"
  local chrom="${requested%%:*}"
  local suffix=""
  if [[ "$requested" == *:* ]]; then
    suffix=":${requested#*:}"
  fi

  if tabix -l "$source" | grep -Fxq "$chrom"; then
    echo "$requested"
  elif [[ "$chrom" == chr* ]] && tabix -l "$source" | grep -Fxq "${chrom#chr}"; then
    echo "${chrom#chr}${suffix}"
  elif [[ "$chrom" != chr* ]] && tabix -l "$source" | grep -Fxq "chr${chrom}"; then
    echo "chr${chrom}${suffix}"
  else
    echo "$requested"
  fi
}

stage_public_heavy_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"
  local region="$4"

  if [[ ! -s "$source" ]]; then
    echo "missing $source; run benchmark/download_public_data.sh igsr-chr22 first" >&2
    exit 2
  fi
  {
    bcftools view -h "$source"
    bcftools view -H -r "$region" "$source" | awk -v max="$records" 'NR <= max'
  } | bgzip -c >"$output"
  tabix -f -p vcf "$output"
}

run_duckdb_query() {
  local parquet="$1"
  local query="$2"
  "$PYTHON" benchmark/query_parquet_duckdb.py "$parquet" --query "$query" --repeats 1
}

run_bcftools_columnar_baseline() {
  local input="$1"
  local query="$2"
  # The helper normalizes bcftools filter, bcftools query, and bcftools view
  # baselines so undefined INFO/DP headers become a comparable zero-count case.
  ./benchmark/bcftools_columnar_baseline.sh "$input" "$query" 1
}

bcftools_columnar_hyperfine_command() {
  local query="$1"
  echo "./benchmark/bcftools_columnar_baseline.sh"
}

query_label() {
  case "$1" in
    row_count) echo "row count" ;;
    qual_gt_30) echo "QUAL > 30" ;;
    dp_gt_40) echo "INFO/DP > 40" ;;
    filter_pass) echo 'FILTER == "PASS"' ;;
    group_by_chrom_filter) echo "grouped counts by CHROM,FILTER" ;;
    *) echo "$1" ;;
  esac
}

require_tool bcftools
require_tool bgzip
require_tool hyperfine
require_tool tabix
require_tool "$PYTHON"
"$PYTHON" benchmark/query_parquet_duckdb.py --check

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi

PUBLIC_REGION="$(resolve_public_region "$REGION" "$IGSR_SOURCE")"

cargo build --release

{
  echo "# VCF-Fast v1.2 Public Parallel And Workflow Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report moves the v1.1 parallel native filter and v1.0 Parquet workflow claims into stronger public-heavy evidence. It compares default native, parallel native, threaded BGZF input, combined threaded BGZF plus parallel native, and \`bcftools\` only where correctness matches."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Public-heavy mode: bounded IGSR chr22 BGZF staging without giant plain VCF intermediates"
  echo "- Dataset source URL: see \`benchmark/download_public_data.sh\` for the pinned IGSR source"
  echo "- Public region: \`$PUBLIC_REGION\`"
  echo "- Public tiers: \`$PUBLIC_TIERS\`"
  echo "- Stress tiers: \`$STRESS_TIERS\`"
  echo "- Native BGZF input threads: \`$BGZF_THREADS\`"
  echo "- Native filter threads: \`$FILTER_THREADS\`"
  echo "- Native filter batch records: \`$BATCH_RECORDS\`"
  echo "- Repeated DuckDB queries: \`$REPEATED_QUERIES\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo "- DuckDB: $("$PYTHON" - <<'PY'
import duckdb
print(duckdb.__version__)
PY
)"
  echo
  echo "## Public-Heavy Parallel Filter Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact default native command | exact parallel native command | exact threaded BGZF command | exact combined command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
} >"$REPORT"

for records in $PUBLIC_TIERS; do
  dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
  stage_public_heavy_dataset "$IGSR_SOURCE" "$dataset" "$records" "$PUBLIC_REGION"
  dataset_size="$(file_size_bytes "$dataset")"
  actual_records="$(count_vcf_records "$dataset")"

  default_out="$OUT_DIR/public-default-qual-${records}.vcf"
  parallel_out="$OUT_DIR/public-parallel-qual-${records}.vcf"
  threaded_out="$OUT_DIR/public-threaded-bgzf-qual-${records}.vcf"
  combined_out="$OUT_DIR/public-combined-qual-${records}.vcf"
  bcftools_out="$OUT_DIR/public-bcftools-qual-${records}.vcf"
  hyperfine_json="$OUT_DIR/hyperfine-public-filter-${records}.json"

  default_command="env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $default_out"
  parallel_command="VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $parallel_out"
  threaded_command="VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $threaded_out"
  combined_command="VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $combined_out"
  competitor_command="bcftools filter -i 'QUAL>30' $dataset -o $bcftools_out"

  env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$default_out"
  VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$parallel_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$threaded_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$combined_out"
  bcftools filter -i 'QUAL>30' "$dataset" -o "$bcftools_out"

  diff -u "$default_out" "$parallel_out" >"$OUT_DIR/equivalence-public-default-parallel-${records}.diff"
  diff -u "$default_out" "$threaded_out" >"$OUT_DIR/equivalence-public-default-threaded-${records}.diff"
  diff -u "$default_out" "$combined_out" >"$OUT_DIR/equivalence-public-default-combined-${records}.diff"
  extract_core_records "$bcftools_out" >"$OUT_DIR/public-bcftools-qual-${records}.records"
  extract_core_records "$default_out" >"$OUT_DIR/public-default-qual-${records}.records"
  extract_core_records "$combined_out" >"$OUT_DIR/public-combined-qual-${records}.records"
  diff -u "$OUT_DIR/public-bcftools-qual-${records}.records" "$OUT_DIR/public-default-qual-${records}.records" >"$OUT_DIR/equivalence-public-default-bcftools-${records}.diff"
  diff -u "$OUT_DIR/public-bcftools-qual-${records}.records" "$OUT_DIR/public-combined-qual-${records}.records" >"$OUT_DIR/equivalence-public-combined-bcftools-${records}.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-default-qual-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-parallel-qual-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-threaded-bgzf-qual-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-combined-qual-${records}.timed.vcf" \
    "bcftools filter -i 'QUAL>30' $dataset -o $OUT_DIR/public-bcftools-qual-${records}.timed.vcf"

  read -r default_mean default_stddev parallel_mean parallel_stddev threaded_mean threaded_stddev combined_mean combined_stddev bcftools_mean bcftools_stddev parallel_speedup threaded_speedup combined_speedup threaded_vs_bcftools combined_vs_bcftools < <(summarize_public_filter_hyperfine "$hyperfine_json")

  default_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-default-${records}.txt" env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-default-qual-${records}.rss.vcf")"
  parallel_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-parallel-${records}.txt" env VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-parallel-qual-${records}.rss.vcf")"
  threaded_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-threaded-${records}.txt" env VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-threaded-bgzf-qual-${records}.rss.vcf")"
  combined_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-combined-${records}.txt" env VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-combined-qual-${records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-bcftools-${records}.txt" bcftools filter -i 'QUAL>30' "$dataset" -o "$OUT_DIR/public-bcftools-qual-${records}.rss.vcf")"

  default_vps="$(variants_per_second "$actual_records" "$default_mean")"
  parallel_vps="$(variants_per_second "$actual_records" "$parallel_mean")"
  threaded_vps="$(variants_per_second "$actual_records" "$threaded_mean")"
  combined_vps="$(variants_per_second "$actual_records" "$combined_mean")"
  bcftools_vps="$(variants_per_second "$actual_records" "$bcftools_mean")"

  claim="correctness matched; inspect measured rows before claiming public parallel win"
  if [[ "$actual_records" -lt 10000 ]]; then
    claim="smoke validation only; no speed claim from sub-10k tier"
  elif "$PYTHON" - "$threaded_mean" "$combined_mean" "${threaded_speedup%x}" <<'PY'
import sys
threaded_mean = float(sys.argv[1].rstrip("s"))
combined_mean = float(sys.argv[2].rstrip("s"))
threaded_speedup = float(sys.argv[3])
raise SystemExit(0 if threaded_speedup > 1 and threaded_mean < combined_mean else 1)
PY
  then
    claim="threaded BGZF input was fastest; combined also beat default but was slower than threaded on this I/O-bound QUAL filter"
  elif "$PYTHON" - "${combined_speedup%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="combined threaded BGZF plus parallel native measured faster than default native on this public-heavy filter"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s |\n' \
    "Public-heavy IGSR QUAL filter" \
    "public-heavy bounded IGSR chr22 BGZF" \
    "$dataset_size" \
    "$actual_records" \
    "$(markdown_cell "$default_command")" \
    "$(markdown_cell "$parallel_command")" \
    "$(markdown_cell "$threaded_command")" \
    "$(markdown_cell "$combined_command")" \
    "$(markdown_cell "$competitor_command")" \
    "default, parallel native, threaded BGZF, and combined native outputs match byte-for-byte; combined and default core records match bcftools filter" \
    "default $default_mean +/- $default_stddev; parallel $parallel_mean +/- $parallel_stddev; threaded $threaded_mean +/- $threaded_stddev; combined $combined_mean +/- $combined_stddev; bcftools $bcftools_mean +/- $bcftools_stddev" \
    "parallel/default $parallel_speedup; threaded/default $threaded_speedup; combined/default $combined_speedup; threaded/bcftools $threaded_vs_bcftools; combined/bcftools $combined_vs_bcftools" \
    "default $default_vps / parallel $parallel_vps / threaded $threaded_vps / combined $combined_vps / bcftools $bcftools_vps" \
    "default $default_rss / parallel $parallel_rss / threaded $threaded_rss / combined $combined_rss / bcftools $bcftools_rss KB" \
    "bounded chr22 region; not whole-genome or all expressions" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Stress Parallel Filter Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact default native command | exact parallel native command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
} >>"$REPORT"

for records in $STRESS_TIERS; do
  dataset="$DATA_DIR/stress-${records}.vcf"
  ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"
  actual_records="$(count_vcf_records "$dataset")"
  default_out="$OUT_DIR/stress-default-any-format-ad-${records}.vcf"
  parallel_out="$OUT_DIR/stress-parallel-any-format-ad-${records}.vcf"
  bcftools_out="$OUT_DIR/stress-bcftools-any-format-ad-${records}.vcf"
  hyperfine_json="$OUT_DIR/hyperfine-stress-filter-${records}.json"

  default_command="env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $default_out"
  parallel_command="VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $parallel_out"
  competitor_command="bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $bcftools_out"

  env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$default_out"
  VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$parallel_out"
  bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$bcftools_out"

  diff -u "$default_out" "$parallel_out" >"$OUT_DIR/equivalence-stress-default-parallel-${records}.diff"
  extract_core_records "$bcftools_out" >"$OUT_DIR/stress-bcftools-any-format-ad-${records}.records"
  extract_core_records "$default_out" >"$OUT_DIR/stress-default-any-format-ad-${records}.records"
  extract_core_records "$parallel_out" >"$OUT_DIR/stress-parallel-any-format-ad-${records}.records"
  diff -u "$OUT_DIR/stress-bcftools-any-format-ad-${records}.records" "$OUT_DIR/stress-default-any-format-ad-${records}.records" >"$OUT_DIR/equivalence-stress-default-bcftools-${records}.diff"
  diff -u "$OUT_DIR/stress-bcftools-any-format-ad-${records}.records" "$OUT_DIR/stress-parallel-any-format-ad-${records}.records" >"$OUT_DIR/equivalence-stress-parallel-bcftools-${records}.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/stress-default-any-format-ad-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/stress-parallel-any-format-ad-${records}.timed.vcf" \
    "bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $OUT_DIR/stress-bcftools-any-format-ad-${records}.timed.vcf"

  read -r default_mean default_stddev parallel_mean parallel_stddev bcftools_mean bcftools_stddev parallel_speedup parallel_vs_bcftools < <(summarize_three_way_hyperfine "$hyperfine_json")

  default_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-default-${records}.txt" env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/stress-default-any-format-ad-${records}.rss.vcf")"
  parallel_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-parallel-${records}.txt" env VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/stress-parallel-any-format-ad-${records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-bcftools-${records}.txt" bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$OUT_DIR/stress-bcftools-any-format-ad-${records}.rss.vcf")"
  default_vps="$(variants_per_second "$actual_records" "$default_mean")"
  parallel_vps="$(variants_per_second "$actual_records" "$parallel_mean")"
  bcftools_vps="$(variants_per_second "$actual_records" "$bcftools_mean")"

  claim="correctness matched; inspect parallel overhead before claiming stress win"
  if [[ "$actual_records" -lt 10000 ]]; then
    claim="smoke validation only; no speed claim from sub-10k tier"
  elif "$PYTHON" - "${parallel_speedup%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="parallel native measured faster than default native on this CPU-heavy stress expression"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s |\n' \
    "Stress ANY FORMAT/AD filter" \
    "deterministic stress VCF" \
    "$dataset_size" \
    "$actual_records" \
    "$(markdown_cell "$default_command")" \
    "$(markdown_cell "$parallel_command")" \
    "$(markdown_cell "$competitor_command")" \
    "parallel native matches default native byte-for-byte and matches bcftools filtered core records" \
    "default $default_mean +/- $default_stddev; parallel $parallel_mean +/- $parallel_stddev; bcftools $bcftools_mean +/- $bcftools_stddev" \
    "parallel/default $parallel_speedup; parallel/bcftools $parallel_vs_bcftools" \
    "default $default_vps / parallel $parallel_vps / bcftools $bcftools_vps" \
    "default $default_rss / parallel $parallel_rss / bcftools $bcftools_rss KB" \
    "synthetic stress CPU-heavy expression; public FORMAT-heavy evidence still pending" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Columnar Workflow Query Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact export command | exact DuckDB command | exact competitor command | correctness result | export mean/stddev | DuckDB repeated query mean/stddev | bcftools repeated scan mean/stddev | query-only speedup | amortized speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |"
} >>"$REPORT"

for records in $PUBLIC_TIERS; do
  dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
  if [[ ! -s "$dataset" ]]; then
    stage_public_heavy_dataset "$IGSR_SOURCE" "$dataset" "$records" "$PUBLIC_REGION"
  fi
  dataset_size="$(file_size_bytes "$dataset")"
  actual_records="$(count_vcf_records "$dataset")"
  parquet_out="$OUT_DIR/variants-public-heavy-${records}.parquet"
  export_command="./target/release/vcf-fast convert $dataset --to parquet -o $parquet_out"
  ./target/release/vcf-fast convert "$dataset" --to parquet -o "$parquet_out"

  for query in qual_gt_30 dp_gt_40 filter_pass group_by_chrom_filter; do
    label="$(query_label "$query")"
    slug="${query//_/-}"
    hyperfine_json="$OUT_DIR/hyperfine-columnar-${slug}-${records}.json"
    duckdb_command="$PYTHON benchmark/query_parquet_duckdb.py $parquet_out --query $query --repeats $REPEATED_QUERIES"
    competitor_command="./benchmark/bcftools_columnar_baseline.sh $dataset $query $REPEATED_QUERIES"
    duckdb_result="$OUT_DIR/duckdb-${slug}-${records}.txt"
    bcftools_result="$OUT_DIR/bcftools-${slug}-${records}.txt"
    run_duckdb_query "$parquet_out" "$query" >"$duckdb_result"
    run_bcftools_columnar_baseline "$dataset" "$query" >"$bcftools_result"
    diff -u "$bcftools_result" "$duckdb_result" >"$OUT_DIR/equivalence-columnar-${slug}-${records}.diff"

    bcftools_hyperfine_prefix="$(bcftools_columnar_hyperfine_command "$query")"
    hyperfine \
      --warmup "$WARMUP" \
      --runs "$RUNS" \
      --export-json "$hyperfine_json" \
      "./target/release/vcf-fast convert $dataset --to parquet -o $OUT_DIR/variants-public-heavy-${slug}-${records}.timed.parquet" \
      "$PYTHON benchmark/query_parquet_duckdb.py $parquet_out --query $query --repeats $REPEATED_QUERIES >/dev/null" \
      "$bcftools_hyperfine_prefix $dataset $query $REPEATED_QUERIES >/dev/null"

    read -r export_mean export_stddev duckdb_mean duckdb_stddev bcftools_mean bcftools_stddev query_speedup amortized_speedup < <(summarize_columnar_hyperfine "$hyperfine_json")
    export_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-columnar-export-${slug}-${records}.txt" ./target/release/vcf-fast convert "$dataset" --to parquet -o "$OUT_DIR/variants-public-heavy-${slug}-${records}.rss.parquet")"
    duckdb_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-columnar-duckdb-${slug}-${records}.txt" "$PYTHON" benchmark/query_parquet_duckdb.py "$parquet_out" --query "$query" --repeats "$REPEATED_QUERIES")"
    bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-columnar-bcftools-${slug}-${records}.txt" ./benchmark/bcftools_columnar_baseline.sh "$dataset" "$query" "$REPEATED_QUERIES")"
    vps="$(variants_per_second "$actual_records" "$duckdb_mean")"

    claim="correctness matched; repeated Parquet workflow needs measured speedup before claim"
    if [[ "$actual_records" -lt 10000 ]]; then
      claim="smoke validation only; no speed claim from sub-10k tier"
    elif "$PYTHON" - "${amortized_speedup%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
    then
      claim="amortized export-plus-repeated-query workflow measured faster than repeated bcftools scans for this query"
    fi

    printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | %s | %s +/- %s | %s +/- %s | %s +/- %s | %s | %s | %s queried variants/sec | export %s / duckdb %s / bcftools %s KB | %s | %s |\n' \
      "export once repeated $label" \
      "public-heavy bounded IGSR chr22 BGZF" \
      "$dataset_size" \
      "$actual_records" \
      "$(markdown_cell "$export_command")" \
      "$(markdown_cell "$duckdb_command")" \
      "$(markdown_cell "$competitor_command")" \
      "DuckDB $label result matches normalized bcftools baseline" \
      "$export_mean" "$export_stddev" \
      "$duckdb_mean" "$duckdb_stddev" \
      "$bcftools_mean" "$bcftools_stddev" \
      "$query_speedup" \
      "$amortized_speedup" \
      "$vps" \
      "$export_rss" "$duckdb_rss" "$bcftools_rss" \
      "selected native Parquet projection only; no BCF/region Parquet path yet" \
      "$claim" >>"$REPORT"
  done
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

echo "v1.2 public parallel workflow report written to $REPORT"
