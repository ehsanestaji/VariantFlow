#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_V23_OUT_DIR:-tests/output/benchmark-results/v23-bgzf-pipeline}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V23_REPORT:-benchmark/reports/v23-bgzf-pipeline-benchmark.md}"
MODE="${VCF_FAST_V23_MODE:-smoke}"
RUNS="${VCF_FAST_V23_RUNS:-1}"
BGZF_THREADS="${VCF_FAST_NATIVE_BGZF_THREADS_BENCH:-4}"
FILTER_THREADS="${VCF_FAST_NATIVE_FILTER_THREADS_BENCH:-4}"
BATCH_RECORDS="${VCF_FAST_NATIVE_FILTER_BATCH_RECORDS_BENCH:-4096}"
QUEUE_BATCHES="${VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES_BENCH:-2}"
BIN="${VCF_FAST_BIN:-target/release/variantflow}"
FAST_EXPR="${VCF_FAST_V23_EXPR:-ANY(FORMAT/AD > 80)}"
BCFTOOLS_EXPR="${VCF_FAST_V23_BCFTOOLS_EXPR:-N_PASS(FMT/AD[*:*]>80)>0}"

case "$MODE" in
  smoke)
    SIZES="${VCF_FAST_V23_SIZES:-100}"
    ;;
  full)
    SIZES="${VCF_FAST_V23_SIZES:-100000 1000000}"
    ;;
  *)
    echo "VCF_FAST_V23_MODE must be smoke or full" >&2
    exit 2
    ;;
esac

if ! [[ "$RUNS" =~ ^[1-9][0-9]*$ ]]; then
  echo "VCF_FAST_V23_RUNS must be a positive integer" >&2
  exit 2
fi

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool for v2.3 BGZF pipeline benchmark: $tool" >&2
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

shell_join() {
  local rendered=()
  local arg
  for arg in "$@"; do
    rendered+=("$(printf "%q" "$arg")")
  done
  printf "%s" "${rendered[*]}"
}

json_field() {
  local json="$1"
  local field="$2"
  python3 - "$json" "$field" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    value = json.load(handle)[sys.argv[2]]
if isinstance(value, float):
    print(f"{value:.6f}")
else:
    print(value)
PY
}

write_report_header() {
  mkdir -p "$(dirname "$REPORT")" "$OUT_DIR" "$DATA_DIR"
  {
    echo "# VariantFlow v2.3 BGZF Pipeline Benchmark"
    echo
    echo "This report is the scaffold for measuring native BGZF decompression and predicate evaluation as separable pipeline stages. It records only rows produced by this harness; no measured rows are checked in by default."
    echo
    echo "## Run Configuration"
    echo
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Mode: \`$MODE\`"
    echo "- Sizes: \`$SIZES\`"
    echo "- Runs per mode: \`$RUNS\`"
    echo "- VariantFlow expression: \`$FAST_EXPR\`"
    echo "- bcftools filter expression: \`$BCFTOOLS_EXPR\`"
    echo "- BGZF worker setting: \`$BGZF_THREADS\`"
    echo "- Predicate worker setting: \`$FILTER_THREADS\`"
    echo "- Predicate batch records: \`$BATCH_RECORDS\`"
    echo "- Predicate queue batches: \`$QUEUE_BATCHES\`"
    echo "- variantflow binary: \`$BIN\`"
    echo "- bcftools: $(tool_version bcftools)"
    echo
    echo "## Measured Rows"
    echo
    echo "| dataset | mode | exact command | wall seconds | peak RSS KB | correctness result | notes |"
    echo "| --- | --- | --- | ---: | ---: | --- | --- |"
  } >"$REPORT"
}

prepare_bgzf_dataset() {
  local records="$1"
  local plain="$DATA_DIR/v23-stress-${records}.vcf"
  local bgzf="${plain}.gz"

  if [[ ! -s "$bgzf" ]]; then
    benchmark/generate_stress_vcf.sh "$plain" "$records"
    bgzip -f "$plain"
    tabix -f -p vcf "$bgzf" >/dev/null 2>&1 || true
  fi

  echo "$bgzf"
}

extract_core_records() {
  local input="$1"
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$input"
}

run_metric() {
  local label="$1"
  shift
  local json="$OUT_DIR/${label}.metrics.json"
  python3 benchmark/command_resource_metrics.py --json-out "$json" -- "$@"
  local exit_code
  exit_code="$(json_field "$json" exit_code)"
  if [[ "$exit_code" != "0" ]]; then
    echo "benchmark command failed for $label with exit code $exit_code" >&2
    exit "$exit_code"
  fi
  echo "$json"
}

append_row() {
  local dataset="$1"
  local mode="$2"
  local command="$3"
  local metrics="$4"
  local correctness="$5"
  local notes="$6"
  local wall_seconds
  local peak_rss

  wall_seconds="$(json_field "$metrics" wall_seconds)"
  peak_rss="$(json_field "$metrics" peak_rss_kb)"
  {
    printf "| %s " "$(markdown_cell "$dataset")"
    printf "| %s " "$(markdown_cell "$mode")"
    printf "| \`%s\` " "$(markdown_cell "$command")"
    printf "| %s " "$wall_seconds"
    printf "| %s " "$peak_rss"
    printf "| %s " "$(markdown_cell "$correctness")"
    printf "| %s |\n" "$(markdown_cell "$notes")"
  } >>"$REPORT"
}

run_variantflow_mode() {
  local dataset_label="$1"
  local mode_name="$2"
  local dataset="$3"
  local output="$4"
  shift 4
  local env_args=("$@")
  local command=(env "${env_args[@]}" "$BIN" filter "$dataset" --where "$FAST_EXPR" -o "$output")
  local metrics

  metrics="$(run_metric "${dataset_label}-${mode_name}" "${command[@]}")"
  echo "$metrics"
}

variantflow_command() {
  local dataset="$1"
  local output="$2"
  shift 2
  local env_args=("$@")
  local command=(env "${env_args[@]}" "$BIN" filter "$dataset" --where "$FAST_EXPR" -o "$output")
  shell_join "${command[@]}"
}

run_bcftools_mode() {
  local dataset_label="$1"
  local dataset="$2"
  local output="$3"
  local command=(bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$output")
  local metrics

  metrics="$(run_metric "${dataset_label}-bcftools-filter" "${command[@]}")"
  echo "$metrics"
}

bcftools_command() {
  local dataset="$1"
  local output="$2"
  local command=(bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$output")
  shell_join "${command[@]}"
}

check_correctness() {
  local forced="$1"
  local bgzf_only="$2"
  local predicate_only="$3"
  local combined="$4"
  local bcftools_output="$5"
  local diff_prefix="$6"

  cmp -s "$forced" "$bgzf_only"
  cmp -s "$forced" "$predicate_only"
  cmp -s "$forced" "$combined"
  extract_core_records "$forced" >"${diff_prefix}.variantflow-core.tsv"
  extract_core_records "$bcftools_output" >"${diff_prefix}.bcftools-core.tsv"
  diff -u "${diff_prefix}.variantflow-core.tsv" "${diff_prefix}.bcftools-core.tsv" >"${diff_prefix}.core.diff"
}

main() {
  require_tool cargo
  require_tool python3
  require_tool bgzip
  require_tool bcftools

  mkdir -p "$OUT_DIR" "$DATA_DIR"
  cargo build --release
  write_report_header

  local records
  for records in $SIZES; do
    local dataset
    local label
    local forced_env
    local bgzf_only_env
    local predicate_only_env
    local combined_env

    dataset="$(prepare_bgzf_dataset "$records")"
    label="stress-format-${records}"
    forced_env=(
      VCF_FAST_NATIVE_BGZF_THREADS=1
      VCF_FAST_NATIVE_FILTER_THREADS=1
      VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS"
      VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$QUEUE_BATCHES"
    )
    bgzf_only_env=(
      VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS"
      VCF_FAST_NATIVE_FILTER_THREADS=1
      VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS"
      VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$QUEUE_BATCHES"
    )
    predicate_only_env=(
      VCF_FAST_NATIVE_BGZF_THREADS=1
      VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS"
      VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS"
      VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$QUEUE_BATCHES"
    )
    combined_env=(
      VCF_FAST_NATIVE_BGZF_THREADS="$BGZF_THREADS"
      VCF_FAST_NATIVE_FILTER_THREADS="$FILTER_THREADS"
      VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$BATCH_RECORDS"
      VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$QUEUE_BATCHES"
    )

    local run_number
    for ((run_number = 1; run_number <= RUNS; run_number++)); do
      local run_label="${label}-run-${run_number}"
      local forced="$OUT_DIR/${run_label}-forced-single.vcf"
      local bgzf_only="$OUT_DIR/${run_label}-bgzf-only.vcf"
      local predicate_only="$OUT_DIR/${run_label}-predicate-only.vcf"
      local combined="$OUT_DIR/${run_label}-combined-pipeline.vcf"
      local bcftools_output="$OUT_DIR/${run_label}-bcftools.vcf"
      local forced_metrics
      local bgzf_only_metrics
      local predicate_only_metrics
      local combined_metrics
      local bcftools_metrics
      local correctness
      local run_suffix="run ${run_number}/${RUNS}"

      forced_metrics="$(run_variantflow_mode "$run_label" "forced-single" "$dataset" "$forced" "${forced_env[@]}")"
      bgzf_only_metrics="$(run_variantflow_mode "$run_label" "bgzf-only" "$dataset" "$bgzf_only" "${bgzf_only_env[@]}")"
      predicate_only_metrics="$(run_variantflow_mode "$run_label" "predicate-only" "$dataset" "$predicate_only" "${predicate_only_env[@]}")"
      combined_metrics="$(run_variantflow_mode "$run_label" "combined-pipeline" "$dataset" "$combined" "${combined_env[@]}")"
      bcftools_metrics="$(run_bcftools_mode "$run_label" "$dataset" "$bcftools_output")"

      check_correctness "$forced" "$bgzf_only" "$predicate_only" "$combined" "$bcftools_output" "$OUT_DIR/${run_label}"
      correctness="VariantFlow modes match byte-for-byte; forced-single matches bcftools filter core records"
      append_row "$label" "forced-single ${run_suffix}" "$(variantflow_command "$dataset" "$forced" "${forced_env[@]}")" "$forced_metrics" "$correctness" "single BGZF and single predicate worker baseline"
      append_row "$label" "bgzf-only ${run_suffix}" "$(variantflow_command "$dataset" "$bgzf_only" "${bgzf_only_env[@]}")" "$bgzf_only_metrics" "$correctness" "parallel BGZF with single predicate worker"
      append_row "$label" "predicate-only ${run_suffix}" "$(variantflow_command "$dataset" "$predicate_only" "${predicate_only_env[@]}")" "$predicate_only_metrics" "$correctness" "single BGZF with parallel predicate workers"
      append_row "$label" "combined-pipeline ${run_suffix}" "$(variantflow_command "$dataset" "$combined" "${combined_env[@]}")" "$combined_metrics" "$correctness" "parallel BGZF and predicate workers"
      append_row "$label" "bcftools filter ${run_suffix}" "$(bcftools_command "$dataset" "$bcftools_output")" "$bcftools_metrics" "$correctness" "competitor baseline"
      {
        echo
        echo "- ${label} ${run_suffix} correctness result: ${correctness}."
      } >>"$REPORT"
    done
  done
}

main "$@"
