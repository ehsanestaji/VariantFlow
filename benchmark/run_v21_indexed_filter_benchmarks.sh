#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

MODE="${VCF_FAST_V21_MODE:-synthetic}"
SIZES="${VCF_FAST_V21_SIZES:-10000 100000 1000000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
if [[ "$MODE" = "public-igsr" ]]; then
  DEFAULT_EXPR="AF > 0.99"
  DEFAULT_BCFTOOLS_EXPR="INFO/AF>0.99"
else
  DEFAULT_EXPR="QUAL > 1000"
  DEFAULT_BCFTOOLS_EXPR="QUAL>1000"
fi
EXPR="${VCF_FAST_V21_EXPR:-$DEFAULT_EXPR}"
BCFTOOLS_EXPR="${VCF_FAST_V21_BCFTOOLS_EXPR:-$DEFAULT_BCFTOOLS_EXPR}"
INDEX_MIN_SKIP_RATE="${VCF_FAST_INDEX_MIN_SKIP_RATE:-0.80}"
INDEX_CHUNK_RECORDS="${VCF_FAST_INDEX_CHUNK_RECORDS:-8192}"
OUT_DIR="${VCF_FAST_V21_OUT_DIR:-${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v21-indexed-filter}}"
DATA_DIR="${OUT_DIR}/data"
REPORT="${VCF_FAST_V21_REPORT:-benchmark/reports/v21-indexed-filter-benchmark.md}"
COMMANDS_REPORT="${OUT_DIR}/v21-indexed-filter-commands.md"
BIN="${VCF_FAST_BIN:-target/release/variantflow}"
PUBLIC_INPUT="${VCF_FAST_V21_PUBLIC_INPUT:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
PUBLIC_SOURCE_URL="https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool: $tool" >&2
    exit 2
  fi
}

require_tool bgzip
require_tool bcftools

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
: >"$COMMANDS_REPORT"

if [[ -z "${VCF_FAST_BIN:-}" ]]; then
  cargo build --release
fi

if [[ ! -x "$BIN" ]]; then
  echo "release binary not found or not executable: $BIN" >&2
  exit 1
fi

shell_command() {
  printf "%q " "$@"
}

measure_peak_rss_kb() {
  local label="$1"
  shift
  if command -v /usr/bin/time >/dev/null 2>&1; then
    if /usr/bin/time -v true >/dev/null 2>&1; then
      /usr/bin/time -v -o "${OUT_DIR}/${label}.time" "$@" >"${OUT_DIR}/${label}.stdout" 2>"${OUT_DIR}/${label}.stderr" || return $?
      awk -F: '/Maximum resident set size/ {gsub(/ /, "", $2); print $2}' "${OUT_DIR}/${label}.time" || true
    else
      /usr/bin/time -l "$@" >"${OUT_DIR}/${label}.stdout" 2>"${OUT_DIR}/${label}.time" || return $?
      awk '/maximum resident set size/ {printf "%.0f\n", $1 / 1024}' "${OUT_DIR}/${label}.time" || true
    fi
  else
    "$@" >"${OUT_DIR}/${label}.stdout"
    echo "n/a"
  fi
}

runtime_mean_stddev() {
  local label="$1"
  local command_text="$2"
  local json="${OUT_DIR}/${label}.hyperfine.json"
  if command -v hyperfine >/dev/null 2>&1; then
    hyperfine --runs "$RUNS" --warmup "$WARMUP" --export-json "$json" "$command_text" >"${OUT_DIR}/${label}.hyperfine.txt"
    python3 - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
result = data["results"][0]
stddev = result.get("stddev") or 0.0
print(f'{result["mean"]:.6f} {stddev:.6f}')
PY
  else
    local start_seconds end_seconds
    start_seconds="$(python3 - <<'PY'
import time
print(f"{time.perf_counter():.9f}")
PY
)"
    bash -lc "$command_text" >"${OUT_DIR}/${label}.runtime.stdout" 2>"${OUT_DIR}/${label}.runtime.stderr"
    end_seconds="$(python3 - <<'PY'
import time
print(f"{time.perf_counter():.9f}")
PY
)"
    python3 - "$start_seconds" "$end_seconds" <<'PY'
import sys

elapsed = float(sys.argv[2]) - float(sys.argv[1])
print(f"{elapsed:.6f} 0.000000")
PY
  fi
}

speedup_ratio() {
  local fast_seconds="$1"
  local baseline_seconds="$2"
  python3 - "$fast_seconds" "$baseline_seconds" <<'PY'
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

json_field() {
  local path="$1"
  local field="$2"
  python3 - "$path" "$field" <<'PY'
import json
import sys

try:
    with open(sys.argv[1], encoding="utf-8") as handle:
        value = json.load(handle).get(sys.argv[2], "n/a")
except FileNotFoundError:
    value = "n/a"
if isinstance(value, bool):
    print(str(value).lower())
else:
    print(value if value is not None else "n/a")
PY
}

skip_rate() {
  local skipped="$1"
  local total="$2"
  python3 - "$skipped" "$total" <<'PY'
import sys

try:
    skipped = float(sys.argv[1])
    total = float(sys.argv[2])
except ValueError:
    print("n/a")
else:
    print("n/a" if total <= 0 else f"{(skipped / total) * 100:.1f}%")
PY
}

claim_decision() {
  local indexed_seconds="$1"
  local default_seconds="$2"
  local bcftools_seconds="$3"
  python3 - "$indexed_seconds" "$default_seconds" "$bcftools_seconds" <<'PY'
import sys

indexed = float(sys.argv[1])
default = float(sys.argv[2])
bcftools = float(sys.argv[3])

if indexed < default and indexed < bcftools:
    print("claim decision: correctness passed; indexed speed win vs default native and bcftools for this measured row")
elif indexed < bcftools:
    print("claim decision: correctness passed; indexed beats bcftools but is slower than default native on this measured row")
elif indexed < default:
    print("claim decision: correctness passed; indexed beats default native but not bcftools on this measured row")
else:
    print("claim decision: correctness passed; no indexed speed claim for this measured row")
PY
}

write_core_records() {
  awk 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5 }'
}

prepare_dataset() {
  local records="$1"

  if [[ "$MODE" = "public-igsr" ]]; then
    prepare_public_igsr_dataset "$records"
    return
  fi

  local plain="${DATA_DIR}/v21-indexed-filter-stress-${records}.vcf"
  local bgzf="${plain}.gz"

  if [[ ! -s "$bgzf" ]]; then
    benchmark/generate_stress_vcf.sh "$plain" "$records"
    bgzip -f "$plain"
  fi

  echo "$bgzf"
}

prepare_public_igsr_dataset() {
  local records="$1"
  local bgzf="${DATA_DIR}/v21-indexed-filter-public-igsr-${records}.vcf.gz"
  local tmp="${bgzf}.tmp"

  if [[ ! -s "$PUBLIC_INPUT" ]]; then
    echo "missing public IGSR input: $PUBLIC_INPUT" >&2
    echo "run: benchmark/download_public_data.sh igsr-chr22" >&2
    exit 2
  fi

  if [[ ! -s "$bgzf" ]]; then
    {
      bcftools view -h "$PUBLIC_INPUT"
      bcftools view -H "$PUBLIC_INPUT" | awk -v limit="$records" 'NR <= limit { print } NR == limit { exit }'
    } | bgzip -c >"$tmp"
    mv "$tmp" "$bgzf"
  fi

  echo "$bgzf"
}

run_default_filter() {
  local dataset="$1"
  local output="$2"
  local index_path="${dataset}.vfi"
  local hidden_index="${index_path}.disabled"
  local status=0

  if [[ -e "$hidden_index" ]]; then
    rm -f "$hidden_index"
  fi
  if [[ -e "$index_path" ]]; then
    mv "$index_path" "$hidden_index"
  fi

  set +e
  "$BIN" filter "$dataset" --where "$EXPR" -o "$output"
  status=$?
  set -e

  if [[ -e "$hidden_index" ]]; then
    mv "$hidden_index" "$index_path"
  fi

  return "$status"
}

run_indexed_filter() {
  local dataset="$1"
  local output="$2"
  local index_report="$3"
  VCF_FAST_INDEX_REPORT="$index_report" VCF_FAST_INDEX_MIN_SKIP_RATE="$INDEX_MIN_SKIP_RATE" "$BIN" filter "$dataset" --where "$EXPR" -o "$output"
}

run_bcftools_filter() {
  local dataset="$1"
  local output="$2"
  bcftools filter -Ov -i "$BCFTOOLS_EXPR" "$dataset" -o /dev/stdout | write_core_records >"$output"
}

{
  echo "# v2.1 Indexed Filter Benchmark"
  echo
  echo "This report measures VariantFlow v2.1 Indexed Filter behavior using BGZF virtual offsets. It compares default native filtering, indexed native filtering, and bcftools filter."
  if [[ "$MODE" = "public-igsr" ]]; then
    echo
    echo "The public mode stages bounded BGZF tiers from the cached 1000 Genomes / IGSR chr22 VCF without writing a plain VCF intermediate. The configured expression is \`${EXPR}\`, which exercises guarded chunk metadata on real public records."
  else
    echo
    echo "The synthetic mode default expression is \`${EXPR}\`, which is designed to skip all deterministic stress chunks because generated QUAL values are 0..99."
  fi
  echo
  echo "Rows outside the configured tiers are not yet measured; keep that caveat attached to any claim decision."
  echo
  echo "## Environment"
  echo
  echo "- sizes: \`${SIZES}\`"
  echo "- mode: \`${MODE}\`"
  echo "- runs: \`${RUNS}\`"
  echo "- warmup: \`${WARMUP}\`"
  echo "- expression: \`${EXPR}\`"
  echo "- bcftools expression: \`${BCFTOOLS_EXPR}\`"
  echo "- index minimum skip rate: \`${INDEX_MIN_SKIP_RATE}\`"
  echo "- index chunk_record_target: \`${INDEX_CHUNK_RECORDS}\`"
  if [[ "$MODE" = "public-igsr" ]]; then
    echo "- dataset source: \`${PUBLIC_SOURCE_URL}\`"
    echo "- cached input: \`${PUBLIC_INPUT}\`"
  else
    echo "- dataset source: deterministic stress generator \`benchmark/generate_stress_vcf.sh\`"
  fi
  echo "- bcftools: \`$(bcftools --version 2>/dev/null | head -1 || echo unavailable)\`"
  if command -v hyperfine >/dev/null 2>&1; then
    echo "- hyperfine: \`$(hyperfine --version 2>/dev/null | head -1 || echo available)\`"
  else
    echo "- hyperfine: not installed; used simple single-run fallback"
  fi
  echo
  echo "## Results"
  echo
  echo "| tier records | chunk_record_target | index action | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | core records | correctness result | guarded indexed runtime mean +/- stddev | default runtime mean +/- stddev | bcftools runtime mean +/- stddev | speedup | guarded variants/sec | peak RSS | claim decision | caveat |"
  echo "| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  dataset="$(prepare_dataset "$records")"
  index_path="${dataset}.vfi"
  default_vcf="${OUT_DIR}/default-output-${records}.vcf"
  indexed_vcf="${OUT_DIR}/indexed-output-${records}.vcf"
  default_core="${OUT_DIR}/default-core-${records}.tsv"
  indexed_core="${OUT_DIR}/indexed-core-${records}.tsv"
  bcftools_core="${OUT_DIR}/bcftools-core-${records}.tsv"
  index_report="${OUT_DIR}/index-report-${records}.json"

  index_cmd="VCF_FAST_INDEX_CHUNK_RECORDS=$(printf "%q" "$INDEX_CHUNK_RECORDS") $(shell_command "$BIN" index "$dataset" -o "$index_path")"

  # variantflow index creates the .vfi sidecar used for BGZF virtual offsets.
  VCF_FAST_INDEX_CHUNK_RECORDS="$INDEX_CHUNK_RECORDS" "$BIN" index "$dataset" -o "$index_path"

  run_default_filter "$dataset" "$default_vcf"
  run_indexed_filter "$dataset" "$indexed_vcf" "$index_report"
  write_core_records <"$default_vcf" >"$default_core"
  write_core_records <"$indexed_vcf" >"$indexed_core"
  run_bcftools_filter "$dataset" "$bcftools_core"

  if cmp -s "$default_vcf" "$indexed_vcf" && cmp -s "$indexed_core" "$bcftools_core"; then
    correctness="default and indexed byte-for-byte match; indexed and bcftools core records match"
    correctness_ok=1
  else
    correctness="correctness result: mismatch; inspect ${OUT_DIR}/*-${records}.vcf and ${OUT_DIR}/*-core-${records}.tsv"
    correctness_ok=0
  fi

  indexed_bench_report="${OUT_DIR}/index-report-${records}-bench.json"
  default_cmd="trap 'mv $(printf "%q" "${index_path}.bench-disabled") $(printf "%q" "$index_path") 2>/dev/null || true' EXIT; mv $(printf "%q" "$index_path") $(printf "%q" "${index_path}.bench-disabled"); $(shell_command "$BIN" filter "$dataset" --where "$EXPR" -o /dev/null)"
  indexed_cmd="VCF_FAST_INDEX_REPORT=$(printf "%q" "$indexed_bench_report") VCF_FAST_INDEX_MIN_SKIP_RATE=$(printf "%q" "$INDEX_MIN_SKIP_RATE") $(shell_command "$BIN" filter "$dataset" --where "$EXPR" -o /dev/null)"
  bcftools_cmd="$(shell_command bcftools filter -Ov -i "$BCFTOOLS_EXPR" "$dataset" -o /dev/null)"

  read -r default_mean default_stddev <<<"$(runtime_mean_stddev "default-${records}" "$default_cmd")"
  read -r indexed_mean indexed_stddev <<<"$(runtime_mean_stddev "indexed-${records}" "$indexed_cmd")"
  read -r bcftools_mean bcftools_stddev <<<"$(runtime_mean_stddev "bcftools-${records}" "$bcftools_cmd")"

  indexed_rss="$(measure_peak_rss_kb "indexed-${records}-rss" env VCF_FAST_INDEX_REPORT="${OUT_DIR}/index-report-${records}-rss.json" VCF_FAST_INDEX_MIN_SKIP_RATE="$INDEX_MIN_SKIP_RATE" "$BIN" filter "$dataset" --where "$EXPR" -o /dev/null || echo "n/a")"
  bcftools_rss="$(measure_peak_rss_kb "bcftools-${records}-rss" bcftools filter -Ov -i "$BCFTOOLS_EXPR" "$dataset" -o /dev/null || echo "n/a")"

  chunks_total="$(json_field "$index_report" chunks_total)"
  chunk_record_target="$(json_field "$index_path" chunk_record_target)"
  chunks_skipped="$(json_field "$index_report" chunks_skipped)"
  records_skipped="$(json_field "$index_report" records_skipped_estimate)"
  indexed_flag="$(json_field "$index_report" indexed)"
  fallback_reason="$(json_field "$index_report" fallback_reason)"
  if [[ "$indexed_flag" = "true" ]]; then
    index_action="used VFI"
  else
    index_action="fell back: ${fallback_reason}"
  fi
  rate="$(skip_rate "$chunks_skipped" "$chunks_total")"
  core_records="$(wc -l <"$indexed_core" | tr -d ' ')"
  speedup="$(speedup_ratio "$indexed_mean" "$default_mean") vs default; $(speedup_ratio "$indexed_mean" "$bcftools_mean") vs bcftools"
  throughput="$(variants_per_second "$records" "$indexed_mean")"
  if [[ "$correctness_ok" = "1" ]]; then
    if [[ "$indexed_flag" = "true" ]]; then
      claim="$(claim_decision "$indexed_mean" "$default_mean" "$bcftools_mean")"
    else
      claim="claim decision: correctness passed; VFI planner fell back to default native because skip estimate was below threshold"
    fi
  else
    claim="claim decision: no speed claim"
  fi
  if [[ "$MODE" = "public-igsr" ]]; then
    caveat="bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered"
  else
    caveat="synthetic stress BGZF only; public-data caveat and broader predicates are not covered"
  fi

  {
    echo "| ${records} | ${chunk_record_target} | ${index_action} | ${chunks_total} | ${chunks_skipped} | ${rate} | ${records_skipped} | ${core_records} | ${correctness} | ${indexed_mean}s +/- ${indexed_stddev}s | ${default_mean}s +/- ${default_stddev}s | ${bcftools_mean}s +/- ${bcftools_stddev}s | ${speedup} | ${throughput} | indexed ${indexed_rss} KB; bcftools ${bcftools_rss} KB | ${claim} | ${caveat} |"
  } >>"$REPORT"

  {
    echo
    echo "### Commands for ${records}"
    echo
    echo "- index: \`${index_cmd}\`"
    echo "- default: \`${default_cmd}\`"
    echo "- indexed: \`${indexed_cmd}\`"
    echo "- bcftools: \`${bcftools_cmd}\`"
  } >>"$COMMANDS_REPORT"
done

{
  echo
  echo "## Exact Commands"
  cat "$COMMANDS_REPORT"
} >>"$REPORT"

echo "wrote ${REPORT}"
