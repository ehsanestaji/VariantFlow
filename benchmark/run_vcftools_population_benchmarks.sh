#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_VCFTOOLS_POPGEN_OUT_DIR:-tests/output/vcftools-popgen-benchmark}"
REPORT="${VCF_FAST_VCFTOOLS_POPGEN_REPORT:-benchmark/reports/vcftools-popgen-parity-benchmark.md}"
INPUT="${VCF_FAST_VCFTOOLS_POPGEN_INPUT:-tests/data/popgen_stats.vcf}"
POP1="${VCF_FAST_VCFTOOLS_POPGEN_POP1:-tests/data/popgen_pop1.txt}"
POP2="${VCF_FAST_VCFTOOLS_POPGEN_POP2:-tests/data/popgen_pop2.txt}"
PUBLIC_INPUT="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT:-}"
PUBLIC_POP1="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_POP1:-}"
PUBLIC_POP2="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_POP2:-}"
PUBLIC_TIERS="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_TIERS:-1000 10000 50000}"
PUBLIC_METADATA="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_METADATA:-tests/data/popgen_sample_metadata.tsv}"
RESOURCE_RUNNER="${VCF_FAST_RESOURCE_RUNNER:-python3 benchmark/command_resource_metrics.py}"
POPULATION_METADATA_HELPER="${VCF_FAST_POPULATION_METADATA_HELPER:-python3 benchmark/vcftools_population_metadata.py}"
# Default tier labels: public cohort 1000, public cohort 10000,
# public cohort 50000. Population sources include real population files when
# provided through VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_POP1/2. Measured rows capture
# peak RSS, CPU seconds, and CPU-hour estimate fields with $RESOURCE_RUNNER.
WINDOW_SIZE="${VCF_FAST_VCFTOOLS_POPGEN_WINDOW_SIZE:-200}"
LD_WINDOW_BP="${VCF_FAST_VCFTOOLS_POPGEN_LD_WINDOW_BP:-500}"
RUNS="${VCF_FAST_VCFTOOLS_POPGEN_RUNS:-3}"
WARMUP="${VCF_FAST_VCFTOOLS_POPGEN_WARMUP:-1}"
VCFTOOLS_DOCKER_IMAGE="${VCF_FAST_VCFTOOLS_DOCKER_IMAGE:-biocontainers/vcftools:v0.1.16-1-deb_cv1}"
DEFAULT_PUBLIC_INPUTS=(
  "tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz"
  "tests/output/benchmark-results/v19-second-public-format-cohort/second-format-cohort-10000.vcf.gz"
  "tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz"
)

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

shell_quote() {
  printf "%q" "$1"
}

join_quoted() {
  local out="" arg
  for arg in "$@"; do
    if [[ -n "$out" ]]; then
      out+=" "
    fi
    out+="$(shell_quote "$arg")"
  done
  printf "%s" "$out"
}

stream_vcf_text() {
  if [[ "$1" == *.gz ]]; then
    gzip -dc "$1"
  else
    cat "$1"
  fi
}

vcftools_input_flag() {
  if [[ "$1" == *.gz ]]; then
    printf -- "--gzvcf"
  else
    printf -- "--vcf"
  fi
}

count_records() {
  stream_vcf_text "$1" | awk 'BEGIN { n=0 } /^#/ { next } { n++ } END { print n }'
}

count_samples() {
  stream_vcf_text "$1" | awk 'BEGIN { FS="\t"; n=0 } /^#CHROM/ { n=NF-9 } END { print n }'
}

input_size() {
  wc -c <"$1" | tr -d ' '
}

json_field() {
  python3 - "$1" "$2" <<'PY'
import json
import sys

path, field = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    data = json.load(handle)
value = data[field]
if isinstance(value, float):
    precision = 9 if field == "cpu_hours" else 6
    print(f"{value:.{precision}f}")
else:
    print(value)
PY
}

run_resource_metrics() {
  local command="$1" json_out="$2"
  $RESOURCE_RUNNER --json-out "$json_out" -- bash -lc "$command"
}

detect_vcftools() {
  if command -v vcftools >/dev/null 2>&1; then
    VCFTOOLS_MODE="local"
    VCFTOOLS_VERSION="$(vcftools --version 2>&1 | head -n 1)"
    return 0
  fi
  if command -v docker >/dev/null 2>&1 && docker image inspect "$VCFTOOLS_DOCKER_IMAGE" >/dev/null 2>&1; then
    VCFTOOLS_MODE="docker"
    VCFTOOLS_VERSION="$(docker run --rm "$VCFTOOLS_DOCKER_IMAGE" vcftools --version 2>&1 | head -n 1)"
    return 0
  fi
  return 1
}

vcftools_shell_prefix() {
  if [[ "${VCFTOOLS_MODE:-}" == "docker" ]]; then
    printf "docker run --rm -v %q:/work -w /work %q vcftools" "$ROOT_DIR" "$VCFTOOLS_DOCKER_IMAGE"
  else
    printf "vcftools"
  fi
}

select_default_public_input() {
  local candidate
  for candidate in "${DEFAULT_PUBLIC_INPUTS[@]}"; do
    if [[ -f "$candidate" ]]; then
      printf "%s" "$candidate"
      return 0
    fi
  done
  return 1
}

prepare_public_biallelic_dataset() {
  local input="$1" output="$2" tier_limit="$3"
  if ! command -v bcftools >/dev/null 2>&1 || ! command -v bgzip >/dev/null 2>&1; then
    echo "blocked: public cohort tier staging requires bcftools and bgzip" >&2
    return 1
  fi

  mkdir -p "$(dirname "$output")"
  local tmp_output
  tmp_output="$(mktemp "${output}.tmp.XXXXXX")"
  set +o pipefail
  bcftools view -m2 -M2 -v snps "$input" \
    | awk -v limit="$tier_limit" '
      /^#/ { print; next }
      seen < limit { print; seen++ }
      seen >= limit { exit }
    ' \
    | bgzip -c >"$tmp_output"
  local statuses=("${PIPESTATUS[@]}")
  set -o pipefail
  if [[ "${statuses[0]}" -ne 0 && "${statuses[0]}" -ne 141 ]]; then
    echo "bcftools view -m2 -M2 failed while staging public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
    echo "failed to write staged public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  if ! mv -f "$tmp_output" "$output"; then
    echo "failed to publish staged public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  printf "%s" "$output"
}

public_population_files() {
  local tier_input="$1" tier_limit="$2"
  if [[ -n "$PUBLIC_POP1" && -n "$PUBLIC_POP2" ]]; then
    printf "%s\t%s\tprovided\tprovided real population files" "$PUBLIC_POP1" "$PUBLIC_POP2"
    return 0
  fi
  $POPULATION_METADATA_HELPER \
    --vcf "$tier_input" \
    --metadata "$PUBLIC_METADATA" \
    --out-prefix "$OUT_DIR/public-cohort-$tier_limit"
}

append_public_pending_row() {
  local tier_limit="$1" blocker="$2"
  printf "| public cohort %s | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | %s | pending | %s |\n" "$tier_limit" "$VCFTOOLS_VERSION" "$blocker" >>"$ROWS_FILE"
}

write_blocker_report() {
  cat >"$REPORT" <<EOF
# VCFtools Population-Genetics Parity Benchmark

Status: blocked. No local \`vcftools\` binary was found, and the default Docker
image \`$VCFTOOLS_DOCKER_IMAGE\` was not cached. Install VCFtools or cache/set
\`VCF_FAST_VCFTOOLS_DOCKER_IMAGE\`, then run \`make bench-vcftools-popgen\`.

Required fields for measured rows: runtime mean, speedup, input size, record
count, sample count, peak RSS KB, CPU seconds, CPU-hour estimate, exact
VariantFlow command, exact VCFtools command, VCFtools version, correctness
result, caveats.

| tier | case | runtime mean | speedup | input size | record count | sample count | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|---|
| fixture | frequency | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | missingness | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | HWE | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | heterozygosity | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | site pi | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | window pi | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | Tajima's D | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | LD | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | Weir-Cockerham Fst | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | pending | pending | pending | pending | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| public cohort 1000 | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: VCFtools unavailable; set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF |
| public cohort 10000 | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: VCFtools unavailable; set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF |
| public cohort 50000 | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: VCFtools unavailable; set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF |

Claim decision: no speed claim. Once measured, only rows whose correctness result
passes may support the cautious claim that VariantFlow matches VCFtools on
supported diploid biallelic parity fixtures.
EOF
}

if ! detect_vcftools; then
  write_blocker_report
  echo "VCFtools unavailable; blocker report written to $REPORT"
  exit 77
fi

cargo build --release
make vcftools-parity

if ! python3 benchmark/check_vcftools_parity.py "${VCF_FAST_VCFTOOLS_OUT_DIR:-tests/output/vcftools-parity}" >/tmp/vcftools-popgen-parity-check.txt 2>&1; then
  cat /tmp/vcftools-popgen-parity-check.txt
  echo "Correctness gate failed; not running VCFtools population benchmark timings."
  exit 1
fi
CORRECTNESS_RESULT="passed: make vcftools-parity; tier outputs checked by benchmark/check_vcftools_parity.py"

VF_BIN="./target/release/variantflow"
VCFTOOLS_PREFIX="$(vcftools_shell_prefix)"
ROWS_FILE="$OUT_DIR/report-rows.md"
: >"$ROWS_FILE"

benchmark_pair() {
  local tier="$1" case_name="$2" dataset="$3" vf_command="$4" vcftools_command="$5" caveats="$6"
  local safe_case
  safe_case="$(
    printf "%s" "$tier-$case_name" \
      | tr '[:upper:]' '[:lower:]' \
      | tr " '" "--" \
      | tr -cd 'a-z0-9._-'
  )"
  local vf_time="$OUT_DIR/${safe_case}.variantflow.time"
  local vcftools_time="$OUT_DIR/${safe_case}.vcftools.time"
  local vf_metrics="$OUT_DIR/${safe_case}.variantflow.resources.json"
  local vcftools_metrics="$OUT_DIR/${safe_case}.vcftools.resources.json"
  local records samples bytes vf_runtime vcftools_runtime speedup
  local vf_peak_rss_kb vcftools_peak_rss_kb vf_cpu_seconds vcftools_cpu_seconds vf_cpu_hours vcftools_cpu_hours
  records="$(count_records "$dataset")"
  samples="$(count_samples "$dataset")"
  bytes="$(input_size "$dataset")"
  hyperfine --warmup "$WARMUP" --runs "$RUNS" \
    --export-json "$OUT_DIR/${safe_case}.hyperfine.json" \
    "$vf_command" "$vcftools_command" >/dev/null
  python3 - "$OUT_DIR/${safe_case}.hyperfine.json" "$vf_time" "$vcftools_time" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)["results"]
vf = data[0]["mean"]
vcf = data[1]["mean"]
open(sys.argv[2], "w", encoding="utf-8").write(f"{vf:.6f}s")
open(sys.argv[3], "w", encoding="utf-8").write(f"{vcf:.6f}s")
PY
  vf_runtime="$(cat "$vf_time")"
  vcftools_runtime="$(cat "$vcftools_time")"
  run_resource_metrics "$vf_command" "$vf_metrics"
  run_resource_metrics "$vcftools_command" "$vcftools_metrics"
  vf_peak_rss_kb="$(json_field "$vf_metrics" peak_rss_kb)"
  vcftools_peak_rss_kb="$(json_field "$vcftools_metrics" peak_rss_kb)"
  vf_cpu_seconds="$(json_field "$vf_metrics" cpu_seconds)"
  vcftools_cpu_seconds="$(json_field "$vcftools_metrics" cpu_seconds)"
  vf_cpu_hours="$(json_field "$vf_metrics" cpu_hours)"
  vcftools_cpu_hours="$(json_field "$vcftools_metrics" cpu_hours)"
  speedup="$(python3 - "$vf_runtime" "$vcftools_runtime" <<'PY'
import sys
vf = float(sys.argv[1].removesuffix("s"))
vcf = float(sys.argv[2].removesuffix("s"))
print(f"{vcf / vf:.2f}x" if vf else "n/a")
PY
)"
  printf "| %s | %s | VariantFlow %s; VCFtools %s | %s | %s bytes | %s | %s | %s | %s | %s | %s | %s | %s | \`%s\` | \`%s\` | %s | %s | %s |\n" \
    "$tier" "$case_name" "$vf_runtime" "$vcftools_runtime" "$speedup" "$bytes" "$records" "$samples" \
    "$vf_peak_rss_kb" "$vcftools_peak_rss_kb" "$vf_cpu_seconds" "$vcftools_cpu_seconds" "$vf_cpu_hours" "$vcftools_cpu_hours" \
    "$vf_command" "$vcftools_command" "$VCFTOOLS_VERSION" "$CORRECTNESS_RESULT" "$caveats" >>"$ROWS_FILE"
}

run_tier() {
  local tier="$1" dataset="$2" pop1="$3" pop2="$4" population_source="${5:-fixture population files}"
  local prefix="$OUT_DIR/$tier"
  mkdir -p "$prefix"
  local vf_base vcftools_base input_flag scope_caveat
  vf_base="$(join_quoted "$VF_BIN")"
  vcftools_base="$VCFTOOLS_PREFIX"
  input_flag="$(vcftools_input_flag "$dataset")"
  if [[ "$tier" == "fixture" ]]; then
    scope_caveat="fixture timing only"
  else
    scope_caveat="public biallelic staged cohort; population source: $population_source"
  fi

  benchmark_pair "$tier" "frequency" "$dataset" \
    "$vf_base freq $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.frq")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --freq --out $(shell_quote "$prefix/vcftools-freq")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "missingness" "$dataset" \
    "$vf_base missingness $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow-missingness")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --missing-site --out $(shell_quote "$prefix/vcftools-missing-site") && $vcftools_base $input_flag $(shell_quote "$dataset") --missing-indv --out $(shell_quote "$prefix/vcftools-missing-indv")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; VCFtools site and individual missingness are two commands; $scope_caveat"
  benchmark_pair "$tier" "HWE" "$dataset" \
    "$vf_base hardy $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.hwe")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --hardy --out $(shell_quote "$prefix/vcftools-hardy")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; exact p-value column is outside current output; $scope_caveat"
  benchmark_pair "$tier" "heterozygosity" "$dataset" \
    "$vf_base het $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.het")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --het --out $(shell_quote "$prefix/vcftools-het")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "site pi" "$dataset" \
    "$vf_base pi $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.sites.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --site-pi --out $(shell_quote "$prefix/vcftools-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "window pi" "$dataset" \
    "$vf_base pi $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.windowed.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --window-pi $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-window-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE; $scope_caveat"
  benchmark_pair "$tier" "Tajima's D" "$dataset" \
    "$vf_base tajima-d $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.Tajima.D")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --TajimaD $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-tajima-d")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE; $scope_caveat"
  benchmark_pair "$tier" "LD" "$dataset" \
    "$vf_base ld $(shell_quote "$dataset") --max-distance $(shell_quote "$LD_WINDOW_BP") -o $(shell_quote "$prefix/variantflow.geno.ld")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --geno-r2 --ld-window-bp $(shell_quote "$LD_WINDOW_BP") --out $(shell_quote "$prefix/vcftools-ld")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; genotype dosage R2; $scope_caveat"
  benchmark_pair "$tier" "Weir-Cockerham Fst" "$dataset" \
    "$vf_base fst $(shell_quote "$dataset") --pop $(shell_quote "$pop1") --pop $(shell_quote "$pop2") --estimator weir-cockerham -o $(shell_quote "$prefix/variantflow.weir.fst")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --weir-fst-pop $(shell_quote "$pop1") --weir-fst-pop $(shell_quote "$pop2") --out $(shell_quote "$prefix/vcftools-weir-fst")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; two population files required; $scope_caveat"
}

run_tier "fixture" "$INPUT" "$POP1" "$POP2"
python3 benchmark/check_vcftools_parity.py "$OUT_DIR/fixture" >/tmp/vcftools-popgen-fixture-check.txt 2>&1 || {
  cat /tmp/vcftools-popgen-fixture-check.txt
  echo "Fixture benchmark outputs failed VCFtools parity checks."
  exit 1
}

if [[ -z "$PUBLIC_INPUT" ]]; then
  PUBLIC_INPUT="$(select_default_public_input || true)"
fi

if [[ -n "$PUBLIC_INPUT" && -f "$PUBLIC_INPUT" ]]; then
  for tier_limit in $PUBLIC_TIERS; do
    tier_name="public cohort $tier_limit"
    tier_staging_error="$OUT_DIR/public-cohort.${tier_limit}.staging.err"
    if ! tier_input="$(prepare_public_biallelic_dataset "$PUBLIC_INPUT" "$OUT_DIR/public-cohort.biallelic.${tier_limit}.vcf.gz" "$tier_limit" 2>"$tier_staging_error")"; then
      tier_blocker="$(tr '\n' ' ' <"$tier_staging_error" | sed 's/[[:space:]]*$//')"
      if [[ -z "$tier_blocker" ]]; then
        tier_blocker="blocked: failed to stage bounded biallelic public cohort tier"
      fi
      append_public_pending_row "$tier_limit" "$tier_blocker; public input found but tier was not measured"
      continue
    fi
    public_pop_files="$(public_population_files "$tier_input" "$tier_limit")"
    IFS=$'\t' read -r tier_pop1 tier_pop2 tier_population_metadata tier_population_source <<<"$public_pop_files"
    if [[ -z "$tier_pop1" || -z "$tier_pop2" || -z "$tier_population_source" ]]; then
      echo "Population metadata helper must return pop1, pop2, metadata, and source fields for $tier_name." >&2
      exit 1
    fi
    run_tier "$tier_name" "$tier_input" "$tier_pop1" "$tier_pop2" "$tier_population_source"
    python3 benchmark/check_vcftools_parity.py "$OUT_DIR/$tier_name" >/tmp/vcftools-popgen-public-check.txt 2>&1 || {
      cat /tmp/vcftools-popgen-public-check.txt
      echo "Public benchmark outputs failed VCFtools parity checks for $tier_name."
      exit 1
    }
  done
else
  for tier_limit in $PUBLIC_TIERS; do
    append_public_pending_row "$tier_limit" "set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF; large artifacts remain ignored"
  done
fi

cat >"$REPORT" <<EOF
# VCFtools Population-Genetics Parity Benchmark

Status: benchmark rows generated by \`make bench-vcftools-popgen\`. When no
public input is provided, the harness uses the first cached default public VCF
if available, stages bounded biallelic public cohort tiers with
\`bcftools view -m2 -M2\`, and derives real population files from
\`$PUBLIC_METADATA\` with \`$POPULATION_METADATA_HELPER\` unless
\`VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_POP1\` and
\`VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_POP2\` provide real population files.
Larger public cohorts remain opt-in through
\`VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT\`.

Runs: \`$RUNS\`; warmup: \`$WARMUP\`; public tiers:
\`$PUBLIC_TIERS\`.

Correctness gate: \`make vcftools-parity\` plus
\`benchmark/check_vcftools_parity.py\` on each measured tier output directory.

Measured rows report hyperfine runtime mean and resource metrics captured by
\`$RESOURCE_RUNNER\` from \`python3 benchmark/command_resource_metrics.py\`.
The resource helper runs each exact VariantFlow and VCFtools command once with
\`--json-out <metrics.json> -- bash -lc "\$command"\` and records peak RSS KB,
CPU seconds, and CPU-hour estimate fields. A resource helper failure fails the
measured row.

The population source is recorded in public-tier caveats for each non-fixture row.

| tier | case | runtime mean | speedup | input size | record count | sample count | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|---|
$(cat "$ROWS_FILE")

Claim decision: correctness-matched fixture rows support only the cautious
statement that VariantFlow matches VCFtools on supported diploid biallelic
parity fixtures. Correctness-matched public rows support measured, scoped
performance statements only for the staged bounded biallelic cohort in this
report. This report does not support a broad VCFtools replacement claim.
EOF

echo "VCFtools population benchmark report written to $REPORT"
