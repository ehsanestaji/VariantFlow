#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_V22_OUT_DIR:-tests/output/benchmark-results/v22-scheduler}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V22_REPORT:-benchmark/reports/v22-scheduler-benchmark.md}"
if [[ "${VCF_FAST_V22_STRESS_TIERS+x}" == "x" ]]; then
  STRESS_TIERS="$VCF_FAST_V22_STRESS_TIERS"
else
  STRESS_TIERS="100000 1000000"
fi
if [[ "${VCF_FAST_V22_PUBLIC_TIERS+x}" == "x" ]]; then
  PUBLIC_TIERS="$VCF_FAST_V22_PUBLIC_TIERS"
else
  PUBLIC_TIERS="1000 10000 50000"
fi
RUNS="${VCF_FAST_V22_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V22_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
BGZF_THREADS="${VCF_FAST_NATIVE_BGZF_THREADS_BENCH:-4}"
FILTER_THREADS="${VCF_FAST_NATIVE_FILTER_THREADS_BENCH:-4}"
BATCH_RECORDS="${VCF_FAST_NATIVE_FILTER_BATCH_RECORDS_BENCH:-4096}"
BIN="${VCF_FAST_BIN:-target/release/variantflow}"
FORMAT_EXPR="${VCF_FAST_V22_EXPR:-ANY(FORMAT/AD > 80)}"
BCFTOOLS_EXPR="${VCF_FAST_V22_BCFTOOLS_EXPR:-N_PASS(FMT/AD[*:*]>80)>0}"
PUBLIC_SOURCE_URL="${VCF_FAST_V22_PUBLIC_SOURCE_URL:-https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz}"
PUBLIC_SOURCE="${VCF_FAST_V22_PUBLIC_SOURCE:-${VCF_FAST_HUMAN_FORMAT_VCF:-$PUBLIC_SOURCE_URL}}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool for v2.2 scheduler benchmark: $tool" >&2
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
  bcftools view -H "$1" | wc -l | tr -d ' '
}

extract_core_records() {
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$1"
}

stream_public_source() {
  case "$PUBLIC_SOURCE" in
    http://*|https://*)
      curl -L --fail --silent --show-error "$PUBLIC_SOURCE" | gzip -cd
      ;;
    *.gz|*.bgz)
      gzip -cd "$PUBLIC_SOURCE"
      ;;
    *)
      cat "$PUBLIC_SOURCE"
      ;;
  esac
}

prepare_stress_bgzf() {
  local records="$1"
  local plain="$DATA_DIR/stress-format-${records}.vcf"
  local bgzf="${plain}.gz"
  if [[ ! -s "$bgzf" ]]; then
    benchmark/generate_stress_vcf.sh "$plain" "$records"
    bgzip -f "$plain"
    tabix -f -p vcf "$bgzf" >/dev/null 2>&1 || true
  fi
  echo "$bgzf"
}

prepare_public_bgzf() {
  local records="$1"
  local bgzf="$DATA_DIR/public-human-format-${records}.vcf.gz"
  if [[ ! -s "$bgzf" ]]; then
    set +e
    stream_public_source | awk -v limit="$records" '
      BEGIN { seen_records = 0 }
      /^#/ { print; next }
      seen_records < limit { print; seen_records++ }
      seen_records >= limit { exit }
    ' | bgzip -c >"$bgzf"
    local statuses=("${PIPESTATUS[@]}")
    set -e
    if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
      rm -f "$bgzf"
      return 1
    fi
    tabix -f -p vcf "$bgzf" >/dev/null 2>&1 || true
  fi
  echo "$bgzf"
}

runtime_summary() {
  local json="$1"
  python3 - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def f(value):
    return f"{float(value or 0):.6f}"

print(" ".join(f(value) for result in results for value in (result["mean"], result.get("stddev"))))
PY
}

speedup_ratio() {
  local fast="$1"
  local baseline="$2"
  python3 - "$fast" "$baseline" <<'PY'
import sys
fast = float(sys.argv[1])
baseline = float(sys.argv[2])
print("n/a" if fast <= 0 else f"{baseline / fast:.2f}x")
PY
}

variants_per_second() {
  local records="$1"
  local seconds="$2"
  python3 - "$records" "$seconds" <<'PY'
import sys
records = float(sys.argv[1])
seconds = float(sys.argv[2])
print("n/a" if seconds <= 0 else f"{records / seconds:.0f}")
PY
}

measure_resource_pair() {
  local label="$1"
  shift
  if command -v /usr/bin/time >/dev/null 2>&1 && /usr/bin/time -v true >/dev/null 2>&1; then
    /usr/bin/time -v -o "$OUT_DIR/${label}.time" "$@" >"$OUT_DIR/${label}.stdout" 2>"$OUT_DIR/${label}.stderr"
    awk -F: '
      /User time/ { gsub(/ /, "", $2); user = $2 }
      /System time/ { gsub(/ /, "", $2); sys = $2 }
      /Maximum resident set size/ { gsub(/ /, "", $2); rss = $2 }
      END {
        if (rss == "") rss = "n/a";
        if (user == "" || sys == "") cpu = "n/a"; else cpu = sprintf("%.6f", user + sys);
        print rss, cpu
      }
    ' "$OUT_DIR/${label}.time"
  else
    "$@" >"$OUT_DIR/${label}.stdout"
    echo "n/a n/a"
  fi
}

write_report_header() {
  {
    echo "# VariantFlow v2.2 Scheduler Benchmark"
    echo
    echo "This report is the evidence gate for the native auto scheduler. It compares forced single-thread, default auto, BGZF-only, predicate-only, combined BGZF+predicate scheduling, and \`bcftools filter\` on FORMAT-heavy BGZF workloads."
    echo
    echo "## Run Configuration"
    echo
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Stress tiers: \`$STRESS_TIERS\`"
    echo "- Public FORMAT-rich tiers: \`$PUBLIC_TIERS\`"
    echo "- FORMAT expression: \`$FORMAT_EXPR\`"
    echo "- bcftools expression: \`$BCFTOOLS_EXPR\`"
    echo "- BGZF worker setting: \`$BGZF_THREADS\`"
    echo "- Predicate worker setting: \`$FILTER_THREADS\`"
    echo "- Predicate batch records: \`$BATCH_RECORDS\`"
    echo "- Repeated runs: \`$RUNS\`"
    echo "- Warmup runs: \`$WARMUP\`"
    echo "- hyperfine: $(tool_version hyperfine)"
    echo "- bcftools: $(tool_version bcftools)"
    echo
    echo "## Measured Rows"
    echo
    echo "| dataset | source | size bytes | record count | exact single-thread command | exact default-auto command | exact BGZF-only command | exact predicate-only command | exact combined command | exact competitor command | correctness result | runtime mean/stddev single/default/bgzf-only/predicate-only/combined/bcftools | speedup combined vs single/default/bcftools | variants/sec combined | peak RSS KB single/default/bgzf-only/predicate-only/combined/bcftools | CPU seconds single/default/bgzf-only/predicate-only/combined/bcftools | caveat | claim decision |"
    echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |"
  } >"$REPORT"
}

run_scheduler_case() {
  local dataset_label="$1"
  local dataset_source="$2"
  local dataset="$3"
  local records="$4"
  local safe_label="$5"
  local size_bytes
  size_bytes="$(file_size_bytes "$dataset")"

  local single_out="$OUT_DIR/${safe_label}-single.vcf"
  local default_out="$OUT_DIR/${safe_label}-default.vcf"
  local bgzf_only_out="$OUT_DIR/${safe_label}-bgzf-only.vcf"
  local predicate_only_out="$OUT_DIR/${safe_label}-predicate-only.vcf"
  local combined_out="$OUT_DIR/${safe_label}-combined.vcf"
  local bcftools_out="$OUT_DIR/${safe_label}-bcftools.vcf"
  local hyperfine_json="$OUT_DIR/${safe_label}.hyperfine.json"

  local single_cmd="VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=1 $BIN filter $dataset --where '$FORMAT_EXPR' -o $single_out"
  local default_cmd="env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $default_out"
  local bgzf_only_cmd="VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=1 $BIN filter $dataset --where '$FORMAT_EXPR' -o $bgzf_only_out"
  local predicate_only_cmd="VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $predicate_only_out"
  local combined_cmd="VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $combined_out"
  local bcftools_cmd="bcftools filter -i '$BCFTOOLS_EXPR' $dataset -o $bcftools_out"

  VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=1 "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$single_out"
  env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$default_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS=1 "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$bgzf_only_out"
  VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$predicate_only_out"
  VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$combined_out"
  bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$bcftools_out"

  diff -u "$single_out" "$default_out" >"$OUT_DIR/${safe_label}-single-default.diff"
  diff -u "$default_out" "$bgzf_only_out" >"$OUT_DIR/${safe_label}-default-bgzf-only.diff"
  diff -u "$default_out" "$predicate_only_out" >"$OUT_DIR/${safe_label}-default-predicate-only.diff"
  diff -u "$default_out" "$combined_out" >"$OUT_DIR/${safe_label}-default-combined.diff"
  extract_core_records "$default_out" >"$OUT_DIR/${safe_label}-default.records"
  extract_core_records "$bcftools_out" >"$OUT_DIR/${safe_label}-bcftools.records"
  diff -u "$OUT_DIR/${safe_label}-bcftools.records" "$OUT_DIR/${safe_label}-default.records" >"$OUT_DIR/${safe_label}-default-bcftools.diff"

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=1 $BIN filter $dataset --where '$FORMAT_EXPR' -o $OUT_DIR/${safe_label}-single.timed.vcf" \
    "env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $OUT_DIR/${safe_label}-default.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=1 $BIN filter $dataset --where '$FORMAT_EXPR' -o $OUT_DIR/${safe_label}-bgzf-only.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $OUT_DIR/${safe_label}-predicate-only.timed.vcf" \
    "VCF_FAST_NATIVE_BGZF_THREADS=$BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=$FILTER_THREADS VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$BATCH_RECORDS $BIN filter $dataset --where '$FORMAT_EXPR' -o $OUT_DIR/${safe_label}-combined.timed.vcf" \
    "bcftools filter -i '$BCFTOOLS_EXPR' $dataset -o $OUT_DIR/${safe_label}-bcftools.timed.vcf"

  read -r single_mean single_std default_mean default_std bgzf_mean bgzf_std predicate_mean predicate_std combined_mean combined_std bcftools_mean bcftools_std < <(runtime_summary "$hyperfine_json")
  read -r single_rss single_cpu < <(measure_resource_pair "${safe_label}-single-rss" env VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=1 "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$OUT_DIR/${safe_label}-single.rss.vcf")
  read -r default_rss default_cpu < <(measure_resource_pair "${safe_label}-default-rss" env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$OUT_DIR/${safe_label}-default.rss.vcf")
  read -r bgzf_rss bgzf_cpu < <(measure_resource_pair "${safe_label}-bgzf-rss" env VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS=1 "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$OUT_DIR/${safe_label}-bgzf-only.rss.vcf")
  read -r predicate_rss predicate_cpu < <(measure_resource_pair "${safe_label}-predicate-rss" env VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$OUT_DIR/${safe_label}-predicate-only.rss.vcf")
  read -r combined_rss combined_cpu < <(measure_resource_pair "${safe_label}-combined-rss" env VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS" VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS" VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS" "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$OUT_DIR/${safe_label}-combined.rss.vcf")
  read -r bcftools_rss bcftools_cpu < <(measure_resource_pair "${safe_label}-bcftools-rss" bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$OUT_DIR/${safe_label}-bcftools.rss.vcf")

  local combined_vs_single combined_vs_default combined_vs_bcftools combined_vps claim
  combined_vs_single="$(speedup_ratio "$combined_mean" "$single_mean")"
  combined_vs_default="$(speedup_ratio "$combined_mean" "$default_mean")"
  combined_vs_bcftools="$(speedup_ratio "$combined_mean" "$bcftools_mean")"
  combined_vps="$(variants_per_second "$records" "$combined_mean")"
  claim="correctness matched; scheduler mode needs full repeated rows before a speed claim"
  if python3 - "$combined_mean" "$default_mean" "$bcftools_mean" <<'PY'
import sys
combined, default, bcftools = map(float, sys.argv[1:])
raise SystemExit(0 if combined < default and combined < bcftools else 1)
PY
  then
    claim="correctness matched; combined scheduler is the measured fastest mode for this row"
  fi

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | `%s` | `%s` | `%s` | %s | %s/%s %s/%s %s/%s %s/%s %s/%s %s/%s | %s / %s / %s | %s | %s/%s/%s/%s/%s/%s | %s/%s/%s/%s/%s/%s | %s | %s |\n' \
    "$dataset_label" \
    "$(markdown_cell "$dataset_source")" \
    "$size_bytes" \
    "$records" \
    "$(markdown_cell "$single_cmd")" \
    "$(markdown_cell "$default_cmd")" \
    "$(markdown_cell "$bgzf_only_cmd")" \
    "$(markdown_cell "$predicate_only_cmd")" \
    "$(markdown_cell "$combined_cmd")" \
    "$(markdown_cell "$bcftools_cmd")" \
    "all VariantFlow modes match default byte-for-byte; default core records match bcftools" \
    "$single_mean" "$single_std" "$default_mean" "$default_std" "$bgzf_mean" "$bgzf_std" "$predicate_mean" "$predicate_std" "$combined_mean" "$combined_std" "$bcftools_mean" "$bcftools_std" \
    "$combined_vs_single" "$combined_vs_default" "$combined_vs_bcftools" \
    "$combined_vps" \
    "$single_rss" "$default_rss" "$bgzf_rss" "$predicate_rss" "$combined_rss" "$bcftools_rss" \
    "$single_cpu" "$default_cpu" "$bgzf_cpu" "$predicate_cpu" "$combined_cpu" "$bcftools_cpu" \
    "FORMAT-heavy BGZF workload; output remains line-preserving for native modes" \
    "$claim" >>"$REPORT"
}

require_tool cargo
require_tool bcftools
require_tool bgzip
require_tool hyperfine
require_tool python3

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
if [[ -z "${VCF_FAST_BIN:-}" ]]; then
  cargo build --release >/dev/null
fi
write_report_header

for records in $STRESS_TIERS; do
  dataset="$(prepare_stress_bgzf "$records")"
  actual_records="$(count_vcf_records "$dataset")"
  run_scheduler_case "stress FORMAT aggregate ${records}" "deterministic stress BGZF from benchmark/generate_stress_vcf.sh" "$dataset" "$actual_records" "stress-${records}"
done

for records in $PUBLIC_TIERS; do
  if dataset="$(prepare_public_bgzf "$records")"; then
    actual_records="$(count_vcf_records "$dataset")"
    run_scheduler_case "public human FORMAT aggregate ${records}" "$PUBLIC_SOURCE" "$dataset" "$actual_records" "public-human-${records}"
  else
    printf '| %s | %s | n/a | %s | n/a | n/a | n/a | n/a | n/a | n/a | deferred: failed to stage public FORMAT-rich BGZF subset | n/a | n/a | n/a | n/a | n/a | public source unavailable or streaming failed | no claim |\n' \
      "public human FORMAT aggregate ${records}" \
      "$(markdown_cell "$PUBLIC_SOURCE")" \
      "$records" >>"$REPORT"
  fi
done

{
  echo
  echo "## Required Report Fields"
  echo
  echo "runtime mean, runtime stddev, speedup, variants/sec, peak RSS KB, CPU seconds, exact commands, correctness result, caveat, and claim decision"
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Output directory: \`$OUT_DIR\`"
  echo "- Hyperfine JSON: \`$OUT_DIR/*.hyperfine.json\`"
  echo "- Equivalence diffs: \`$OUT_DIR/*.diff\`"
} >>"$REPORT"

echo "v2.2 scheduler benchmark report written to $REPORT"
