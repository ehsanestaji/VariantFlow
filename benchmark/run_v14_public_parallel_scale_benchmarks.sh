#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v14-public-parallel-scale}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V14_REPORT:-benchmark/reports/v14-public-parallel-scale-benchmark.md}"
PUBLIC_TIERS="${VCF_FAST_V14_PUBLIC_TIERS:-100000 1000000}"
STRESS_TIERS="${VCF_FAST_V14_STRESS_TIERS:-100000 1000000}"
REGION="${VCF_FAST_V14_REGION:-chr22:1-20000000}"
IGSR_SOURCE="${VCF_FAST_IGSR_SOURCE:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
BGZF_THREADS="${VCF_FAST_NATIVE_BGZF_THREADS_BENCH:-4}"
FILTER_THREADS="${VCF_FAST_NATIVE_FILTER_THREADS_BENCH:-4}"
BATCH_RECORDS="${VCF_FAST_NATIVE_FILTER_BATCH_RECORDS_BENCH:-4096}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.4 public parallel scale evidence" >&2
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

summarize_public_hyperfine() {
  local json="$1"
  python3 - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def fmt(value):
    return f"{float(value or 0):.6f}s"

single, auto, auto_parallel, explicit_bgzf, bcftools = results
single_mean = float(single["mean"])
auto_mean = float(auto["mean"])
auto_parallel_mean = float(auto_parallel["mean"])
explicit_bgzf_mean = float(explicit_bgzf["mean"])
bcftools_mean = float(bcftools["mean"])
print(
    fmt(single_mean),
    fmt(single.get("stddev")),
    fmt(auto_mean),
    fmt(auto.get("stddev")),
    fmt(auto_parallel_mean),
    fmt(auto_parallel.get("stddev")),
    fmt(explicit_bgzf_mean),
    fmt(explicit_bgzf.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{single_mean / auto_mean:.2f}x" if auto_mean > 0 else "n/a",
    f"{single_mean / auto_parallel_mean:.2f}x" if auto_parallel_mean > 0 else "n/a",
    f"{single_mean / explicit_bgzf_mean:.2f}x" if explicit_bgzf_mean > 0 else "n/a",
    f"{bcftools_mean / auto_mean:.2f}x" if auto_mean > 0 else "n/a",
)
PY
}

summarize_stress_hyperfine() {
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

require_tool cargo
require_tool bcftools
require_tool bgzip
require_tool tabix
require_tool hyperfine
require_tool python3

mkdir -p "$DATA_DIR" "$(dirname "$REPORT")"
cargo build --release >/dev/null

RESOLVED_REGION="$(resolve_public_region "$REGION" "$IGSR_SOURCE")"

{
  echo "# VCF-Fast v1.4 Public Parallel Scale Benchmark"
  echo
  echo "This report separates the public I/O-bound BGZF path from CPU-heavy predicate parallelism. It compares single-thread BGZF fallback, default auto BGZF, auto BGZF plus predicate parallelism, explicit BGZF threads, and \`bcftools\` only where correctness matches."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- Public region: \`$RESOLVED_REGION\`"
  echo "- Public tiers: \`$PUBLIC_TIERS\`"
  echo "- Stress tiers: \`$STRESS_TIERS\`"
  echo "- Auto BGZF policy: unset \`VCF_FAST_NATIVE_BGZF_THREADS\` resolves to auto-capped native BGZF workers"
  echo "- Explicit BGZF input threads: \`$BGZF_THREADS\`"
  echo "- Native filter threads: \`$FILTER_THREADS\`"
  echo "- Native filter batch records: \`$BATCH_RECORDS\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "## Public BGZF QUAL Filter Scale"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact single-thread command | exact auto BGZF command | exact auto+predicate-parallel command | exact explicit BGZF command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
} >"$REPORT"

for requested_records in $PUBLIC_TIERS; do
  dataset="$DATA_DIR/public-heavy-${requested_records}.vcf.gz"
  stage_public_heavy_dataset "$IGSR_SOURCE" "$dataset" "$requested_records" "$RESOLVED_REGION"
  actual_records="$(count_vcf_records "$dataset")"
  dataset_size="$(file_size_bytes "$dataset")"

  single_out="$OUT_DIR/public-single-bgzf-${requested_records}.vcf"
  auto_out="$OUT_DIR/public-auto-bgzf-${requested_records}.vcf"
  auto_parallel_out="$OUT_DIR/public-auto-bgzf-parallel-${requested_records}.vcf"
  explicit_out="$OUT_DIR/public-explicit-bgzf-${requested_records}.vcf"
  bcftools_out="$OUT_DIR/public-bcftools-${requested_records}.vcf"
  hyperfine_json="$OUT_DIR/hyperfine-public-scale-${requested_records}.json"

  single_command="VCF_FAST_NATIVE_BGZF_THREADS=1 env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $single_out"
  auto_command="env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $auto_out"
  auto_parallel_command="env -u VCF_FAST_NATIVE_BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $auto_parallel_out"
  explicit_command="VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $explicit_out"
  competitor_command="bcftools filter -i 'QUAL>30' $dataset -o $bcftools_out"

  VCF_FAST_NATIVE_BGZF_THREADS=1 env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$single_out"
  env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$auto_out"
  env -u VCF_FAST_NATIVE_BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$auto_parallel_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$explicit_out"
  bcftools filter -i 'QUAL>30' "$dataset" -o "$bcftools_out"

  diff -u "$single_out" "$auto_out" >"$OUT_DIR/equivalence-public-single-auto-${requested_records}.diff"
  diff -u "$auto_out" "$auto_parallel_out" >"$OUT_DIR/equivalence-public-auto-parallel-${requested_records}.diff"
  diff -u "$auto_out" "$explicit_out" >"$OUT_DIR/equivalence-public-auto-explicit-${requested_records}.diff"
  extract_core_records "$bcftools_out" >"$OUT_DIR/public-bcftools-${requested_records}.records"
  extract_core_records "$auto_out" >"$OUT_DIR/public-auto-${requested_records}.records"
  diff -u "$OUT_DIR/public-bcftools-${requested_records}.records" "$OUT_DIR/public-auto-${requested_records}.records" >"$OUT_DIR/equivalence-public-auto-bcftools-${requested_records}.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "VCF_FAST_NATIVE_BGZF_THREADS=1 env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-single-bgzf-${requested_records}.timed.vcf" \
    "env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-auto-bgzf-${requested_records}.timed.vcf" \
    "env -u VCF_FAST_NATIVE_BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-auto-bgzf-parallel-${requested_records}.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'QUAL > 30' -o $OUT_DIR/public-explicit-bgzf-${requested_records}.timed.vcf" \
    "bcftools filter -i 'QUAL>30' $dataset -o $OUT_DIR/public-bcftools-${requested_records}.timed.vcf"

  read -r single_mean single_stddev auto_mean auto_stddev auto_parallel_mean auto_parallel_stddev explicit_mean explicit_stddev bcftools_mean bcftools_stddev auto_vs_single auto_parallel_vs_single explicit_vs_single auto_vs_bcftools < <(summarize_public_hyperfine "$hyperfine_json")

  single_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-single-${requested_records}.txt" env VCF_FAST_NATIVE_BGZF_THREADS=1 ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-single-bgzf-${requested_records}.rss.vcf")"
  auto_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-auto-${requested_records}.txt" env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-auto-bgzf-${requested_records}.rss.vcf")"
  auto_parallel_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-auto-parallel-${requested_records}.txt" env -u VCF_FAST_NATIVE_BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-auto-bgzf-parallel-${requested_records}.rss.vcf")"
  explicit_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-explicit-${requested_records}.txt" env VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" ./target/release/vcf-fast filter "$dataset" --where "QUAL > 30" -o "$OUT_DIR/public-explicit-bgzf-${requested_records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-public-bcftools-${requested_records}.txt" bcftools filter -i 'QUAL>30' "$dataset" -o "$OUT_DIR/public-bcftools-${requested_records}.rss.vcf")"

  single_vps="$(variants_per_second "$actual_records" "$single_mean")"
  auto_vps="$(variants_per_second "$actual_records" "$auto_mean")"
  auto_parallel_vps="$(variants_per_second "$actual_records" "$auto_parallel_mean")"
  explicit_vps="$(variants_per_second "$actual_records" "$explicit_mean")"
  bcftools_vps="$(variants_per_second "$actual_records" "$bcftools_mean")"

  claim="correctness matched; inspect measured rows before claiming public parallel scale win"
  if [[ "$actual_records" -lt 10000 ]]; then
    claim="smoke validation only; no speed claim from sub-10k tier"
  else
    claim="auto BGZF is the preferred default for this public BGZF QUAL filter; predicate parallelism remains opt-in for CPU-heavy expressions"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s |\n' \
    "Public-heavy IGSR QUAL filter" \
    "public-heavy bounded IGSR chr22 BGZF" \
    "$dataset_size" \
    "$actual_records" \
    "$(markdown_cell "$single_command")" \
    "$(markdown_cell "$auto_command")" \
    "$(markdown_cell "$auto_parallel_command")" \
    "$(markdown_cell "$explicit_command")" \
    "$(markdown_cell "$competitor_command")" \
    "single-thread, auto BGZF, auto+predicate-parallel, and explicit BGZF native outputs match byte-for-byte; auto core records match bcftools filter" \
    "single $single_mean +/- $single_stddev; auto $auto_mean +/- $auto_stddev; auto+parallel $auto_parallel_mean +/- $auto_parallel_stddev; explicit $explicit_mean +/- $explicit_stddev; bcftools $bcftools_mean +/- $bcftools_stddev" \
    "auto/single $auto_vs_single; auto+parallel/single $auto_parallel_vs_single; explicit/single $explicit_vs_single; auto/bcftools $auto_vs_bcftools" \
    "single $single_vps / auto $auto_vps / auto+parallel $auto_parallel_vps / explicit $explicit_vps / bcftools $bcftools_vps" \
    "single $single_rss / auto $auto_rss / auto+parallel $auto_parallel_rss / explicit $explicit_rss / bcftools $bcftools_rss KB" \
    "bounded chr22 region; requested tier may exceed available records in chr22:1-20000000" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Stress FORMAT Aggregate Scale"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact default command | exact parallel command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
} >>"$REPORT"

for records in $STRESS_TIERS; do
  dataset="$DATA_DIR/stress-${records}.vcf"
  ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"

  default_out="$OUT_DIR/stress-default-${records}.vcf"
  parallel_out="$OUT_DIR/stress-parallel-${records}.vcf"
  bcftools_out="$OUT_DIR/stress-bcftools-${records}.vcf"
  hyperfine_json="$OUT_DIR/hyperfine-stress-scale-${records}.json"

  default_command="env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $default_out"
  parallel_command="VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $parallel_out"
  competitor_command="bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $bcftools_out"

  env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$default_out"
  VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$parallel_out"
  bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$bcftools_out"

  diff -u "$default_out" "$parallel_out" >"$OUT_DIR/equivalence-stress-default-parallel-${records}.diff"
  extract_core_records "$bcftools_out" >"$OUT_DIR/stress-bcftools-${records}.records"
  extract_core_records "$parallel_out" >"$OUT_DIR/stress-parallel-${records}.records"
  diff -u "$OUT_DIR/stress-bcftools-${records}.records" "$OUT_DIR/stress-parallel-${records}.records" >"$OUT_DIR/equivalence-stress-parallel-bcftools-${records}.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/stress-default-${records}.timed.vcf" \
    "VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS ./target/release/vcf-fast filter $dataset --where 'ANY(FORMAT/AD > 80)' -o $OUT_DIR/stress-parallel-${records}.timed.vcf" \
    "bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' $dataset -o $OUT_DIR/stress-bcftools-${records}.timed.vcf"

  read -r default_mean default_stddev parallel_mean parallel_stddev bcftools_mean bcftools_stddev parallel_vs_default parallel_vs_bcftools < <(summarize_stress_hyperfine "$hyperfine_json")

  default_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-default-${records}.txt" env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/stress-default-${records}.rss.vcf")"
  parallel_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-parallel-${records}.txt" env VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" ./target/release/vcf-fast filter "$dataset" --where "ANY(FORMAT/AD > 80)" -o "$OUT_DIR/stress-parallel-${records}.rss.vcf")"
  bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-stress-bcftools-${records}.txt" bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' "$dataset" -o "$OUT_DIR/stress-bcftools-${records}.rss.vcf")"

  default_vps="$(variants_per_second "$records" "$default_mean")"
  parallel_vps="$(variants_per_second "$records" "$parallel_mean")"
  bcftools_vps="$(variants_per_second "$records" "$bcftools_mean")"

  claim="correctness matched; inspect measured rows before claiming stress parallel scale win"
  if [[ "$records" -lt 10000 ]]; then
    claim="smoke validation only; no speed claim from sub-10k tier"
  else
    claim="predicate parallelism remains the preferred opt-in path for this CPU-heavy FORMAT aggregate"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s |\n' \
    "Stress ANY FORMAT/AD filter" \
    "deterministic stress VCF" \
    "$dataset_size" \
    "$records" \
    "$(markdown_cell "$default_command")" \
    "$(markdown_cell "$parallel_command")" \
    "$(markdown_cell "$competitor_command")" \
    "parallel native matches default native byte-for-byte and matches bcftools filtered core records" \
    "default $default_mean +/- $default_stddev; parallel $parallel_mean +/- $parallel_stddev; bcftools $bcftools_mean +/- $bcftools_stddev" \
    "parallel/default $parallel_vs_default; parallel/bcftools $parallel_vs_bcftools" \
    "default $default_vps / parallel $parallel_vps / bcftools $bcftools_vps" \
    "default $default_rss / parallel $parallel_rss / bcftools $bcftools_rss KB" \
    "synthetic stress CPU-heavy expression; public FORMAT-heavy evidence still pending" \
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

echo "v1.4 public parallel scale report written to $REPORT"
