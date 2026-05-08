#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_V22_MATRIX_OUT_DIR:-tests/output/benchmark-results/v22-scheduler-matrix}"
DATA_DIR="${VCF_FAST_V22_MATRIX_DATA_DIR:-tests/output/benchmark-results/v22-scheduler/data}"
REPORT="${VCF_FAST_V22_MATRIX_REPORT:-benchmark/reports/v22-scheduler-matrix.md}"
RUNS="${VCF_FAST_V22_MATRIX_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V22_MATRIX_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
STRESS_RECORDS="${VCF_FAST_V22_MATRIX_STRESS_RECORDS:-1000000}"
PUBLIC_RECORDS="${VCF_FAST_V22_MATRIX_PUBLIC_RECORDS:-50000}"
QUEUE_DEPTHS="${VCF_FAST_V22_MATRIX_QUEUE_DEPTHS:-1 2 4}"
BATCH_RECORDS_SET="${VCF_FAST_V22_MATRIX_BATCH_RECORDS:-2048 4096 8192}"
THREAD_PAIRS="${VCF_FAST_V22_MATRIX_THREAD_PAIRS:-2:2 4:4 6:4 4:6}"
BIN="${VCF_FAST_BIN:-target/release/variantflow}"
FORMAT_EXPR="${VCF_FAST_V22_EXPR:-ANY(FORMAT/AD > 80)}"
BCFTOOLS_EXPR="${VCF_FAST_V22_BCFTOOLS_EXPR:-N_PASS(FMT/AD[*:*]>80)>0}"
PUBLIC_SOURCE_URL="${VCF_FAST_V22_PUBLIC_SOURCE_URL:-https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz}"
PUBLIC_SOURCE="${VCF_FAST_V22_PUBLIC_SOURCE:-${VCF_FAST_HUMAN_FORMAT_VCF:-$PUBLIC_SOURCE_URL}}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool for v2.2 scheduler matrix: $tool" >&2
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
    mkdir -p "$DATA_DIR"
    benchmark/generate_stress_vcf.sh "$plain" "$records"
    bgzip -f "$plain"
    tabix -f -p vcf "$bgzf" >/dev/null 2>&1 || true
  fi
  echo "$bgzf"
}

prepare_public_bgzf() {
  local records="$1"
  local bgzf="$DATA_DIR/public-human-format-${records}.vcf.gz"
  if [[ -s "$bgzf" ]] && ! bcftools view -H "$bgzf" >/dev/null 2>&1; then
    rm -f "$bgzf" "$bgzf.tbi" "$bgzf.csi"
  fi
  if [[ ! -s "$bgzf" ]]; then
    mkdir -p "$DATA_DIR"
    set +e
    stream_public_source | python3 -c '
import sys

limit = int(sys.argv[1])
records = 0
stdout = sys.stdout.buffer
for line in sys.stdin.buffer:
    if line.startswith(b"#"):
        stdout.write(line)
        continue
    if records >= limit:
        break
    stdout.write(line)
    records += 1
' "$records" | bgzip -c >"$bgzf"
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
    result = json.load(handle)["results"][0]

print(f"{float(result['mean']):.6f} {float(result.get('stddev') or 0):.6f}")
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
  elif command -v /usr/bin/time >/dev/null 2>&1 && /usr/bin/time -l true >/dev/null 2>&1; then
    /usr/bin/time -l "$@" >"$OUT_DIR/${label}.stdout" 2>"$OUT_DIR/${label}.time"
    awk '
      NR == 1 && $2 == "real" && $4 == "user" && $6 == "sys" {
        cpu = sprintf("%.6f", $3 + $5)
      }
      /maximum resident set size/ {
        rss = sprintf("%.0f", $1 / 1024)
      }
      END {
        if (rss == "") rss = "n/a";
        if (cpu == "") cpu = "n/a";
        print rss, cpu
      }
    ' "$OUT_DIR/${label}.time"
  else
    "$@" >"$OUT_DIR/${label}.stdout"
    echo "n/a n/a"
  fi
}

run_timed_command() {
  local label="$1"
  local command="$2"
  local json="$OUT_DIR/${label}.hyperfine.json"
  hyperfine --warmup "$WARMUP" --runs "$RUNS" --export-json "$json" "$command" >/dev/null
  runtime_summary "$json"
}

write_report_header() {
  {
    echo "# VariantFlow v2.2 Scheduler Queue And Thread Matrix"
    echo
    echo "Focused matrix for tuning native FORMAT-heavy BGZF filtering after the v2.2 scheduler evidence run. Timed runs write to \`/dev/null\`; one correctness run per row writes a temporary VCF and compares it to default native output."
    echo
    echo "## Run Configuration"
    echo
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Stress records: \`$STRESS_RECORDS\`"
    echo "- Public human FORMAT records: \`$PUBLIC_RECORDS\`"
    echo "- Queue depths: \`$QUEUE_DEPTHS\`"
    echo "- Batch records: \`$BATCH_RECORDS_SET\`"
    echo "- BGZF/filter thread pairs: \`$THREAD_PAIRS\`"
    echo "- FORMAT expression: \`$FORMAT_EXPR\`"
    echo "- bcftools expression: \`$BCFTOOLS_EXPR\`"
    echo "- Repeated runs: \`$RUNS\`"
    echo "- Warmup runs: \`$WARMUP\`"
    echo "- hyperfine: $(tool_version hyperfine)"
    echo "- bcftools: $(tool_version bcftools)"
    echo
    echo "## Matrix Rows"
    echo
    echo "| dataset | source | size bytes | records | queue batches | batch records | BGZF threads | filter threads | runtime mean/stddev | speedup vs default/bcftools | variants/sec | peak RSS KB | CPU seconds | correctness result | caveat |"
    echo "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- | ---: | ---: | ---: | --- | --- |"
  } >"$REPORT"
}

run_dataset_matrix() {
  local dataset_label="$1"
  local dataset_source="$2"
  local dataset="$3"
  local safe_label="$4"
  local records size_bytes baseline_out bcftools_out baseline_records bcftools_records default_json bcftools_json

  records="$(count_vcf_records "$dataset")"
  size_bytes="$(file_size_bytes "$dataset")"
  baseline_out="$OUT_DIR/${safe_label}-default.vcf"
  bcftools_out="$OUT_DIR/${safe_label}-bcftools.vcf"
  baseline_records="$OUT_DIR/${safe_label}-default.records"
  bcftools_records="$OUT_DIR/${safe_label}-bcftools.records"

  env -u VCF_FAST_NATIVE_BGZF_THREADS \
    -u VCF_FAST_NATIVE_FILTER_THREADS \
    -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS \
    -u VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES \
    "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$baseline_out"
  bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$bcftools_out"
  extract_core_records "$baseline_out" >"$baseline_records"
  extract_core_records "$bcftools_out" >"$bcftools_records"
  diff -u "$bcftools_records" "$baseline_records" >"$OUT_DIR/${safe_label}-default-bcftools.diff"

  read -r default_mean default_std < <(run_timed_command \
    "${safe_label}-default" \
    "env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS -u VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES $BIN filter $dataset --where '$FORMAT_EXPR' -o /dev/null")
  read -r bcftools_mean bcftools_std < <(run_timed_command \
    "${safe_label}-bcftools" \
    "bcftools filter -i '$BCFTOOLS_EXPR' $dataset -o /dev/null")

  for queue in $QUEUE_DEPTHS; do
    for batch in $BATCH_RECORDS_SET; do
      for pair in $THREAD_PAIRS; do
        local bgzf_threads="${pair%%:*}"
        local filter_threads="${pair##*:}"
        local row_label="${safe_label}-q${queue}-b${batch}-z${bgzf_threads}-f${filter_threads}"
        local tmp_out="$OUT_DIR/${row_label}.vcf"
        local diff_out="$OUT_DIR/${row_label}.diff"
        local cmd mean std rss cpu speed_default speed_bcftools vps

        VCF_FAST_NATIVE_BGZF_THREADS="$bgzf_threads" \
          VCF_FAST_NATIVE_FILTER_THREADS="$filter_threads" \
          VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$batch" \
          VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$queue" \
          "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o "$tmp_out"
        diff -u "$baseline_out" "$tmp_out" >"$diff_out"
        rm -f "$tmp_out"

        cmd="VCF_FAST_NATIVE_BGZF_THREADS=$bgzf_threads VCF_FAST_NATIVE_FILTER_THREADS=$filter_threads VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=$batch VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES=$queue $BIN filter $dataset --where '$FORMAT_EXPR' -o /dev/null"
        read -r mean std < <(run_timed_command "$row_label" "$cmd")
        read -r rss cpu < <(measure_resource_pair \
          "${row_label}-rss" \
          env VCF_FAST_NATIVE_BGZF_THREADS="$bgzf_threads" \
          VCF_FAST_NATIVE_FILTER_THREADS="$filter_threads" \
          VCF_FAST_NATIVE_FILTER_BATCH_RECORDS="$batch" \
          VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES="$queue" \
          "$BIN" filter "$dataset" --where "$FORMAT_EXPR" -o /dev/null)
        speed_default="$(speedup_ratio "$mean" "$default_mean")"
        speed_bcftools="$(speedup_ratio "$mean" "$bcftools_mean")"
        vps="$(variants_per_second "$records" "$mean")"

        printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s/%s | %s / %s | %s | %s | %s | %s | %s |\n' \
          "$dataset_label" \
          "$(markdown_cell "$dataset_source")" \
          "$size_bytes" \
          "$records" \
          "$queue" \
          "$batch" \
          "$bgzf_threads" \
          "$filter_threads" \
          "$mean" "$std" \
          "$speed_default" "$speed_bcftools" \
          "$vps" \
          "$rss" \
          "$cpu" \
          "matches default byte-for-byte; default core records match bcftools" \
          "FORMAT-heavy BGZF matrix; timed output goes to /dev/null" >>"$REPORT"
      done
    done
  done

  {
    echo
    echo "### ${dataset_label} Baselines"
    echo
    echo "- default native runtime mean/stddev: \`${default_mean}/${default_std}\`"
    echo "- bcftools runtime mean/stddev: \`${bcftools_mean}/${bcftools_std}\`"
    echo "- default native output: \`$baseline_out\`"
    echo "- default-vs-bcftools core diff: \`$OUT_DIR/${safe_label}-default-bcftools.diff\`"
  } >>"$REPORT"
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

stress_dataset="$(prepare_stress_bgzf "$STRESS_RECORDS")"
run_dataset_matrix "stress FORMAT aggregate ${STRESS_RECORDS}" "deterministic stress BGZF from benchmark/generate_stress_vcf.sh" "$stress_dataset" "stress-${STRESS_RECORDS}"

public_dataset="$(prepare_public_bgzf "$PUBLIC_RECORDS")"
run_dataset_matrix "public human FORMAT aggregate ${PUBLIC_RECORDS}" "$PUBLIC_SOURCE" "$public_dataset" "public-human-${PUBLIC_RECORDS}"

{
  echo
  echo "## Required Report Fields"
  echo
  echo "runtime mean, runtime stddev, speedup, variants/sec, peak RSS KB, CPU seconds, queue depth, batch size, thread caps, exact tuning variables, correctness result, and caveat"
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Output directory: \`$OUT_DIR\`"
  echo "- Hyperfine JSON: \`$OUT_DIR/*.hyperfine.json\`"
  echo "- Equivalence diffs: \`$OUT_DIR/*.diff\`"
} >>"$REPORT"

echo "v2.2 scheduler matrix report written to $REPORT"
