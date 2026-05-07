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
WINDOW_SIZE="${VCF_FAST_VCFTOOLS_POPGEN_WINDOW_SIZE:-200}"
LD_WINDOW_BP="${VCF_FAST_VCFTOOLS_POPGEN_LD_WINDOW_BP:-500}"
RUNS="${VCF_FAST_VCFTOOLS_POPGEN_RUNS:-3}"
WARMUP="${VCF_FAST_VCFTOOLS_POPGEN_WARMUP:-1}"
PUBLIC_RECORD_LIMIT="${VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_RECORD_LIMIT:-1000}"
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

derive_population_files() {
  local dataset="$1" prefix="$2"
  local pop1="$prefix.auto-pop1.txt"
  local pop2="$prefix.auto-pop2.txt"
  stream_vcf_text "$dataset" \
    | awk -v pop1="$pop1" -v pop2="$pop2" '
      BEGIN { FS = "\t" }
      /^#CHROM/ {
        if (NF < 13) {
          printf "public cohort requires at least four samples for auto populations; found %d\n", NF - 9 > "/dev/stderr"
          exit 2
        }
        print $10 > pop1
        print $11 > pop1
        print $12 > pop2
        print $13 > pop2
        exit
      }
    '
  printf "%s\t%s" "$pop1" "$pop2"
}

prepare_public_biallelic_dataset() {
  local input="$1" output="$2"
  if ! command -v bcftools >/dev/null 2>&1 || ! command -v bgzip >/dev/null 2>&1; then
    printf "%s" "$input"
    return 0
  fi

  if [[ -f "$output" ]]; then
    printf "%s" "$output"
    return 0
  fi

  mkdir -p "$(dirname "$output")"
  set +o pipefail
  bcftools view -m2 -M2 -v snps "$input" \
    | awk -v limit="$PUBLIC_RECORD_LIMIT" '
      /^#/ { print; next }
      seen < limit { print; seen++ }
      seen >= limit { exit }
    ' \
    | bgzip -c >"$output"
  local statuses=("${PIPESTATUS[@]}")
  set -o pipefail
  if [[ "${statuses[0]}" -ne 0 && "${statuses[0]}" -ne 141 ]]; then
    echo "bcftools view -m2 -M2 failed while staging public biallelic dataset" >&2
    return 1
  fi
  if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
    echo "failed to write staged public biallelic dataset" >&2
    return 1
  fi
  printf "%s" "$output"
}

write_blocker_report() {
  cat >"$REPORT" <<EOF
# VCFtools Population-Genetics Parity Benchmark

Status: blocked. No local \`vcftools\` binary was found, and the default Docker
image \`$VCFTOOLS_DOCKER_IMAGE\` was not cached. Install VCFtools or cache/set
\`VCF_FAST_VCFTOOLS_DOCKER_IMAGE\`, then run \`make bench-vcftools-popgen\`.

Required fields for measured rows: runtime, speedup, input size, record count,
sample count, exact VariantFlow command, exact VCFtools command, VCFtools
version, correctness result, caveats.

| tier | case | runtime | speedup | input size | record count | sample count | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---:|---:|---:|---|---|---|---|---|
| fixture | frequency | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | missingness | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | HWE | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | heterozygosity | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | site pi | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | window pi | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | Tajima's D | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | LD | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| fixture | Weir-Cockerham Fst | pending | pending | $(input_size "$INPUT") bytes | $(count_records "$INPUT") | $(count_samples "$INPUT") | pending | pending | unavailable | pending | blocked: VCFtools unavailable |
| public cohort pending | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | pending | pending | set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF |

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
  local records samples bytes vf_runtime vcftools_runtime speedup
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
  speedup="$(python3 - "$vf_runtime" "$vcftools_runtime" <<'PY'
import sys
vf = float(sys.argv[1].removesuffix("s"))
vcf = float(sys.argv[2].removesuffix("s"))
print(f"{vcf / vf:.2f}x" if vf else "n/a")
PY
)"
  printf "| %s | %s | VariantFlow %s; VCFtools %s | %s | %s bytes | %s | %s | \`%s\` | \`%s\` | %s | %s | %s |\n" \
    "$tier" "$case_name" "$vf_runtime" "$vcftools_runtime" "$speedup" "$bytes" "$records" "$samples" \
    "$vf_command" "$vcftools_command" "$VCFTOOLS_VERSION" "$CORRECTNESS_RESULT" "$caveats" >>"$ROWS_FILE"
}

run_tier() {
  local tier="$1" dataset="$2" pop1="$3" pop2="$4"
  local prefix="$OUT_DIR/$tier"
  mkdir -p "$prefix"
  local vf_base vcftools_base input_flag scope_caveat
  vf_base="$(join_quoted "$VF_BIN")"
  vcftools_base="$VCFTOOLS_PREFIX"
  input_flag="$(vcftools_input_flag "$dataset")"
  if [[ "$tier" == "fixture" ]]; then
    scope_caveat="fixture timing only"
  else
    scope_caveat="public biallelic staged cohort"
  fi

  benchmark_pair "$tier" "frequency" "$dataset" \
    "$vf_base freq $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.frq")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --freq --out $(shell_quote "$prefix/vcftools-freq")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "missingness" "$dataset" \
    "$vf_base missingness $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow-missingness")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --missing-site --out $(shell_quote "$prefix/vcftools-missing-site") && $vcftools_base $input_flag $(shell_quote "$dataset") --missing-indv --out $(shell_quote "$prefix/vcftools-missing-indv")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; VCFtools site and individual missingness are two commands"
  benchmark_pair "$tier" "HWE" "$dataset" \
    "$vf_base hardy $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.hwe")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --hardy --out $(shell_quote "$prefix/vcftools-hardy")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; exact p-value column is outside current output"
  benchmark_pair "$tier" "heterozygosity" "$dataset" \
    "$vf_base het $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.het")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --het --out $(shell_quote "$prefix/vcftools-het")" \
    "matches VCFtools on supported diploid biallelic parity fixtures"
  benchmark_pair "$tier" "site pi" "$dataset" \
    "$vf_base pi $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.sites.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --site-pi --out $(shell_quote "$prefix/vcftools-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures"
  benchmark_pair "$tier" "window pi" "$dataset" \
    "$vf_base pi $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.windowed.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --window-pi $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-window-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE"
  benchmark_pair "$tier" "Tajima's D" "$dataset" \
    "$vf_base tajima-d $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.Tajima.D")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --TajimaD $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-tajima-d")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE"
  benchmark_pair "$tier" "LD" "$dataset" \
    "$vf_base ld $(shell_quote "$dataset") --max-distance $(shell_quote "$LD_WINDOW_BP") -o $(shell_quote "$prefix/variantflow.geno.ld")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --geno-r2 --ld-window-bp $(shell_quote "$LD_WINDOW_BP") --out $(shell_quote "$prefix/vcftools-ld")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; genotype dosage R2"
  benchmark_pair "$tier" "Weir-Cockerham Fst" "$dataset" \
    "$vf_base fst $(shell_quote "$dataset") --pop $(shell_quote "$pop1") --pop $(shell_quote "$pop2") --estimator weir-cockerham -o $(shell_quote "$prefix/variantflow.weir.fst")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --weir-fst-pop $(shell_quote "$pop1") --weir-fst-pop $(shell_quote "$pop2") --out $(shell_quote "$prefix/vcftools-weir-fst")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; two population files required"
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
  PUBLIC_INPUT="$(prepare_public_biallelic_dataset "$PUBLIC_INPUT" "$OUT_DIR/public-cohort.biallelic.${PUBLIC_RECORD_LIMIT}.vcf.gz")"
  if [[ -z "$PUBLIC_POP1" || -z "$PUBLIC_POP2" ]]; then
    public_pop_files="$(derive_population_files "$PUBLIC_INPUT" "$OUT_DIR/public-cohort")"
    PUBLIC_POP1="${public_pop_files%%$'\t'*}"
    PUBLIC_POP2="${public_pop_files#*$'\t'}"
  fi
  run_tier "public cohort" "$PUBLIC_INPUT" "$PUBLIC_POP1" "$PUBLIC_POP2"
  python3 benchmark/check_vcftools_parity.py "$OUT_DIR/public cohort" >/tmp/vcftools-popgen-public-check.txt 2>&1 || {
    cat /tmp/vcftools-popgen-public-check.txt
    echo "Public benchmark outputs failed VCFtools parity checks."
    exit 1
  }
else
  printf "| public cohort pending | all population-genetics cases | pending | pending | pending | pending | pending | pending | pending | %s | pending | set VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT to a cached public VCF; large artifacts remain ignored |\n" "$VCFTOOLS_VERSION" >>"$ROWS_FILE"
fi

cat >"$REPORT" <<EOF
# VCFtools Population-Genetics Parity Benchmark

Status: benchmark rows generated by \`make bench-vcftools-popgen\`. When no
public input is provided, the harness uses the cached
\`human-format-cohort-1000.vcf.gz\` if available, stages a bounded biallelic
subset with \`bcftools view -m2 -M2\`, and derives two small population files
from the sample header. Larger public cohorts remain opt-in through
\`VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT\`.

Runs: \`$RUNS\`; warmup: \`$WARMUP\`; public record limit:
\`$PUBLIC_RECORD_LIMIT\`.

Correctness gate: \`make vcftools-parity\` plus
\`benchmark/check_vcftools_parity.py\` on each measured tier output directory.

| tier | case | runtime | speedup | input size | record count | sample count | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---:|---:|---:|---|---|---|---|---|
$(cat "$ROWS_FILE")

Claim decision: correctness-matched fixture rows support only the cautious
statement that VariantFlow matches VCFtools on supported diploid biallelic
parity fixtures. Correctness-matched public rows support measured, scoped
performance statements only for the staged bounded biallelic cohort in this
report. This report does not support a broad VCFtools replacement claim.
EOF

echo "VCFtools population benchmark report written to $REPORT"
