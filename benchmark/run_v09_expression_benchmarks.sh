#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v09-expression}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V09_REPORT:-benchmark/reports/v09-expression-parity-benchmark.md}"
SIZES="${VCF_FAST_V09_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
INFO_FIELDS="${VCF_FAST_STRESS_INFO_FIELDS:-40}"
SAMPLES="${VCF_FAST_STRESS_SAMPLES:-16}"
SAMPLE_NAME="${VCF_FAST_V09_SAMPLE:-SAMPLE_001}"

CASES=(
  "Arbitrary INFO numeric|INFO/UNUSED7 > 300|INFO/UNUSED7>300|"
  "Selected arbitrary FORMAT/AD|FORMAT/AD > 30|FMT/AD[0:*]>30|$SAMPLE_NAME"
  "ANY sample aggregate FORMAT/AD|ANY(FORMAT/AD > 80)|N_PASS(FMT/AD[*:*]>80)>0|"
  "ALL sample aggregate FORMAT/DP|ALL(FORMAT/DP > 20)|N_PASS(FMT/DP>20)==N_SAMPLES|"
)

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v0.9 expression evidence" >&2
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

slugify() {
  echo "$1" | tr '[:upper:] /' '[:lower:]--' | tr -cd '[:alnum:]-'
}

run_fast_filter() {
  local dataset="$1"
  local sample="$2"
  local expression="$3"
  local output="$4"

  if [[ -n "$sample" ]]; then
    ./target/release/vcf-fast filter "$dataset" --sample "$sample" --where "$expression" -o "$output"
  else
    ./target/release/vcf-fast filter "$dataset" --where "$expression" -o "$output"
  fi
}

run_bcftools_filter() {
  local dataset="$1"
  local expression="$2"
  local output="$3"

  bcftools filter -i "$expression" "$dataset" -o "$output"
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
  echo "# VCF-Fast v0.9 Expression Parity Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report tracks correctness and performance for v0.9 expression parity cases. Runtime wins are claimed only for measured rows whose filtered core records match the stated \`bcftools filter\` baseline. No runtime win is claimed outside the measured rows below. The native scope is arbitrary \`INFO/<KEY>\`, selected-sample \`FORMAT/<KEY>\` with \`--sample\`, and \`ANY(FORMAT/<KEY>)\` / \`ALL(FORMAT/<KEY>)\` sample aggregate predicates."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Dataset source: deterministic synthetic stress data from \`benchmark/generate_stress_vcf.sh\`"
  echo "- Dataset shape: stress INFO fields=${INFO_FIELDS}, samples=${SAMPLES}, FORMAT=GT:DP:GQ:AD"
  echo "- Record tiers: \`${SIZES}\`"
  echo "- Repeated runs: \`${RUNS}\`"
  echo "- Warmup runs: \`${WARMUP}\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "## Measured Native Expression Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact VCF-Fast command | exact competitor command | competitor version | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  dataset="$DATA_DIR/v09-expression-stress-${records}.vcf"
  ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
  dataset_size="$(file_size_bytes "$dataset")"

  for case_spec in "${CASES[@]}"; do
    IFS='|' read -r case_name fast_expr bcftools_expr sample <<<"$case_spec"
    case_slug="$(slugify "$case_name")"
    fast_out="$OUT_DIR/fast-${case_slug}-${records}.vcf"
    bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.vcf"
    fast_records="$OUT_DIR/fast-${case_slug}-${records}.records"
    bcftools_records="$OUT_DIR/bcftools-${case_slug}-${records}.records"
    diff_out="$OUT_DIR/equivalence-${case_slug}-${records}.diff"
    hyperfine_json="$OUT_DIR/hyperfine-${case_slug}-${records}.json"

    fast_command="./target/release/vcf-fast filter $dataset"
    if [[ -n "$sample" ]]; then
      fast_command="$fast_command --sample $sample"
    fi
    fast_command="$fast_command --where '$fast_expr' -o $fast_out"
    competitor_command="bcftools filter -i '$bcftools_expr' $dataset -o $bcftools_out"

    run_fast_filter "$dataset" "$sample" "$fast_expr" "$fast_out"
    run_bcftools_filter "$dataset" "$bcftools_expr" "$bcftools_out"
    extract_core_records "$fast_out" >"$fast_records"
    extract_core_records "$bcftools_out" >"$bcftools_records"
    diff -u "$bcftools_records" "$fast_records" >"$diff_out"

    fast_timed="$OUT_DIR/fast-${case_slug}-${records}.timed.vcf"
    bcftools_timed="$OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf"
    fast_timed_command="./target/release/vcf-fast filter $dataset"
    if [[ -n "$sample" ]]; then
      fast_timed_command="$fast_timed_command --sample $sample"
    fi
    fast_timed_command="$fast_timed_command --where '$fast_expr' -o $fast_timed"
    bcftools_timed_command="bcftools filter -i '$bcftools_expr' $dataset -o $bcftools_timed"

    hyperfine \
      --warmup "$WARMUP" \
      --runs "$RUNS" \
      --export-json "$hyperfine_json" \
      "$fast_timed_command" \
      "$bcftools_timed_command"

    read -r fast_mean fast_stddev bcftools_mean bcftools_stddev speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
    fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$2" ]]; then ./target/release/vcf-fast filter "$1" --sample "$2" --where "$3" -o "$4"; else ./target/release/vcf-fast filter "$1" --where "$3" -o "$4"; fi' _ "$dataset" "$sample" "$fast_expr" "$OUT_DIR/fast-${case_slug}-${records}.rss.vcf")"
    bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bcftools filter -i "$bcftools_expr" "$dataset" -o "$OUT_DIR/bcftools-${case_slug}-${records}.rss.vcf")"
    fast_vps="$(variants_per_second "$records" "$fast_mean")"
    bcftools_vps="$(variants_per_second "$records" "$bcftools_mean")"

    caveat="synthetic stress expression evidence; public v0.9 expression rows still pending"
    claim="measured faster on this deterministic stress expression case"
    speed_value="${speedup%x}"
    if python3 - "$speed_value" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) < 1 else 1)
PY
    then
      claim="correctness matched; optimization needed before claiming a win"
    fi

    printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s +/- %s vs %s +/- %s | %s | %s / %s | %s / %s KB | %s | %s |\n' \
      "$case_name" \
      "deterministic stress VCF" \
      "$dataset_size" \
      "$records" \
      "$(markdown_cell "\`$fast_command\`")" \
      "$(markdown_cell "\`$competitor_command\`")" \
      "$(tool_version bcftools)" \
      "matches bcftools filtered core records" \
      "$fast_mean" "$fast_stddev" "$bcftools_mean" "$bcftools_stddev" \
      "$speedup" \
      "$fast_vps" "$bcftools_vps" \
      "$fast_peak_rss_kb" "$bcftools_peak_rss_kb" \
      "$caveat" \
      "$claim" >>"$REPORT"
  done
done

{
  echo
  echo "## Required Report Fields"
  echo
  echo "- dataset source"
  echo "- dataset size"
  echo "- record count"
  echo "- exact VCF-Fast command"
  echo "- exact competitor command"
  echo "- competitor version"
  echo "- correctness result"
  echo "- runtime mean and standard deviation"
  echo "- speedup"
  echo "- variants per second"
  echo "- peak RSS"
  echo "- caveat"
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Working datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-*.json\`"
  echo "- Peak RSS files: \`$OUT_DIR/rss-*.txt\`"
  echo "- Equivalence diffs: \`$OUT_DIR/equivalence-*.diff\`"
} >>"$REPORT"

echo "v0.9 expression evidence report written to $REPORT"
