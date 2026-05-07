#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_V17_TRUE_POP_OUT_DIR:-tests/output/v17-true-population-evidence}"
REPORT="${VCF_FAST_V17_TRUE_POP_REPORT:-benchmark/reports/v17-true-public-population-evidence.md}"
PUBLIC_INPUT="${VCF_FAST_V17_TRUE_POP_INPUT:-}"
PUBLIC_METADATA="${VCF_FAST_V17_TRUE_POP_METADATA:-tests/output/public-data/igsr-1000g-sample-metadata.tsv}"
PUBLIC_TIERS="${VCF_FAST_V17_TRUE_POP_TIERS:-10000 50000 100000}"
PUBLIC_GROUPS="${VCF_FAST_V17_TRUE_POP_GROUPS:-AFR:EUR}"
PUBLIC_GROUP_LEVEL="${VCF_FAST_V17_TRUE_POP_GROUP_LEVEL:-superpopulation}"
POPULATION_HELPER="${VCF_FAST_V17_TRUE_POP_HELPER:-python3 benchmark/igsr_population_files.py}"
RESOURCE_RUNNER="${VCF_FAST_V17_TRUE_POP_RESOURCE_RUNNER:-${VCF_FAST_RESOURCE_RUNNER:-python3 benchmark/command_resource_metrics.py}}"
RUNS="${VCF_FAST_V17_TRUE_POP_RUNS:-3}"
WARMUP="${VCF_FAST_V17_TRUE_POP_WARMUP:-1}"
WINDOW_SIZE="${VCF_FAST_V17_TRUE_POP_WINDOW_SIZE:-200}"
LD_WINDOW_BP="${VCF_FAST_V17_TRUE_POP_LD_WINDOW_BP:-500}"
VCFTOOLS_DOCKER_IMAGE="${VCF_FAST_VCFTOOLS_DOCKER_IMAGE:-biocontainers/vcftools:v0.1.16-1-deb_cv1}"

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

markdown_cell() {
  local value="$1"
  value="${value//$'\n'/ }"
  value="${value//$'\r'/ }"
  value="${value//|/\\|}"
  printf "%s" "$value"
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

actual_records() {
  stream_vcf_text "$1" | awk 'BEGIN { n=0 } /^#/ { next } { n++ } END { print n }'
}

count_samples() {
  stream_vcf_text "$1" | awk 'BEGIN { FS="\t"; n=0 } /^#CHROM/ { n=NF-9 } END { print n }'
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

write_blocked_report() {
  local reason="$1"
  cat >"$REPORT" <<EOF
# VariantFlow v1.7 True Public Population Evidence

Status: blocked. $reason

Set \`VCF_FAST_V17_TRUE_POP_INPUT\` to a cached 1000 Genomes / IGSR VCF/BCF and
\`VCF_FAST_V17_TRUE_POP_METADATA\` to official population metadata with sample,
population, and superpopulation columns.

This report requires official population metadata, actual record count, sample
count, runtime mean, speedup, VariantFlow/VCFtools peak RSS KB, CPU seconds,
CPU-hour estimate, exact commands, VCFtools version, correctness result, and
caveats before any scoped performance claim. It uses no header-fallback
population files.

| tier | case | actual record count | sample count | population metadata source | runtime mean | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|---|
| public cohort 10000 | frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, Weir-Cockerham Fst | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: official IGSR metadata and cached public input required; no header-fallback |
| public cohort 50000 | frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, Weir-Cockerham Fst | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: official IGSR metadata and cached public input required; no header-fallback |
| public cohort 100000 | frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, Weir-Cockerham Fst | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: official IGSR metadata and cached public input required; no header-fallback |

Claim decision: no broad VCFtools replacement claim. This report does not support a broad VCFtools replacement claim.
EOF
}

if [[ -z "$PUBLIC_INPUT" || ! -f "$PUBLIC_INPUT" || ! -f "$PUBLIC_METADATA" ]]; then
  write_blocked_report "Missing cached public VCF/BCF input or official population metadata."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi

if ! command -v bcftools >/dev/null 2>&1 || ! command -v bgzip >/dev/null 2>&1; then
  write_blocked_report "True public cohort staging requires local bcftools and bgzip before benchmark rows can be generated."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi

prepare_true_public_biallelic_dataset() {
  local input="$1" output="$2" tier_limit="$3"
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
    echo "bcftools view -m2 -M2 -v snps failed while staging true public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
    echo "failed to write true public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  mv -f "$tmp_output" "$output"
  printf "%s" "$output"
}

public_population_files() {
  local tier_input="$1" tier_limit="$2"
  $POPULATION_HELPER \
    --vcf "$tier_input" \
    --metadata "$PUBLIC_METADATA" \
    --groups "$PUBLIC_GROUPS" \
    --group-level "$PUBLIC_GROUP_LEVEL" \
    --out-prefix "$OUT_DIR/public-cohort-$tier_limit"
}

if ! detect_vcftools; then
  write_blocked_report "VCFtools is unavailable locally and Docker image $VCFTOOLS_DOCKER_IMAGE is not cached."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi

if [[ "$VCFTOOLS_MODE" == "docker" ]]; then
  write_blocked_report "True population evidence currently requires local vcftools for parity regeneration; Docker-only VCFtools mode is blocked."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi

cargo build --release
make vcftools-parity

if ! python3 benchmark/check_vcftools_parity.py "${VCF_FAST_VCFTOOLS_OUT_DIR:-tests/output/vcftools-parity}" >/tmp/v17-true-public-population-parity-check.txt 2>&1; then
  cat /tmp/v17-true-public-population-parity-check.txt
  echo "Correctness gate failed; not running true public population benchmark timings."
  exit 1
fi
CORRECTNESS_RESULT="passed: make vcftools-parity; tier outputs checked by benchmark/check_vcftools_parity.py"

VF_BIN="./target/release/variantflow"
VCFTOOLS_PREFIX="$(vcftools_shell_prefix)"
ROWS_FILE="$OUT_DIR/report-rows.md"
: >"$ROWS_FILE"

append_pending_row() {
  local tier_limit="$1" blocker="$2"
  local safe_vcftools_version safe_blocker
  safe_vcftools_version="$(markdown_cell "$VCFTOOLS_VERSION")"
  safe_blocker="$(markdown_cell "$blocker")"
  printf "| public cohort %s | frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, Weir-Cockerham Fst | pending | pending | official IGSR metadata; no header-fallback | pending | pending | pending | pending | pending | pending | pending | pending | pending | pending | %s | pending | %s |\n" \
    "$tier_limit" "$safe_vcftools_version" "$safe_blocker" >>"$ROWS_FILE"
}

benchmark_pair() {
  local tier="$1" case_name="$2" dataset="$3" metadata_source="$4" vf_command="$5" vcftools_command="$6" caveats="$7"
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
  local records samples vf_runtime vcftools_runtime speedup
  local vf_peak_rss_kb vcftools_peak_rss_kb vf_cpu_seconds vcftools_cpu_seconds vf_cpu_hours vcftools_cpu_hours
  local safe_tier safe_case_name safe_metadata_source safe_vf_command safe_vcftools_command safe_vcftools_version safe_correctness safe_caveats
  records="$(actual_records "$dataset")"
  samples="$(count_samples "$dataset")"
  hyperfine --warmup "$WARMUP" --runs "$RUNS" \
    --export-json "$OUT_DIR/${safe_case}.hyperfine.json" \
    "$vf_command" "$vcftools_command" >/dev/null
  python3 - "$OUT_DIR/${safe_case}.hyperfine.json" "$vf_time" "$vcftools_time" <<'PY'
import json
import sys

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
  safe_tier="$(markdown_cell "$tier")"
  safe_case_name="$(markdown_cell "$case_name")"
  safe_metadata_source="$(markdown_cell "$metadata_source")"
  safe_vf_command="$(markdown_cell "$vf_command")"
  safe_vcftools_command="$(markdown_cell "$vcftools_command")"
  safe_vcftools_version="$(markdown_cell "$VCFTOOLS_VERSION")"
  safe_correctness="$(markdown_cell "$CORRECTNESS_RESULT")"
  safe_caveats="$(markdown_cell "$caveats")"
  printf "| %s | %s | %s | %s | %s | VariantFlow %s; VCFtools %s | %s | %s | %s | %s | %s | %s | %s | %s | \`%s\` | \`%s\` | %s | %s | %s |\n" \
    "$safe_tier" "$safe_case_name" "$records" "$samples" "$safe_metadata_source" "$vf_runtime" "$vcftools_runtime" "$speedup" \
    "$vf_peak_rss_kb" "$vcftools_peak_rss_kb" "$vf_cpu_seconds" "$vcftools_cpu_seconds" "$vf_cpu_hours" "$vcftools_cpu_hours" \
    "$safe_vf_command" "$safe_vcftools_command" "$safe_vcftools_version" "$safe_correctness" "$safe_caveats" >>"$ROWS_FILE"
}

run_tier() {
  local tier_limit="$1" dataset="$2" pop1="$3" pop2="$4" population_source="$5"
  local tier="public cohort $tier_limit"
  local prefix="$OUT_DIR/$tier"
  mkdir -p "$prefix"
  local vf_base vcftools_base input_flag actual_count scope_caveat
  vf_base="$(join_quoted "$VF_BIN")"
  vcftools_base="$VCFTOOLS_PREFIX"
  input_flag="$(vcftools_input_flag "$dataset")"
  actual_count="$(actual_records "$dataset")"
  scope_caveat="true public biallelic human cohort; population metadata source: $population_source; no header-fallback; requested tier $tier_limit; actual records $actual_count"

  benchmark_pair "$tier" "frequency" "$dataset" "$population_source" \
    "$vf_base freq $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.frq")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --freq --out $(shell_quote "$prefix/vcftools-freq")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "missingness" "$dataset" "$population_source" \
    "$vf_base missingness $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow-missingness")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --missing-site --out $(shell_quote "$prefix/vcftools-missing-site") && $vcftools_base $input_flag $(shell_quote "$dataset") --missing-indv --out $(shell_quote "$prefix/vcftools-missing-indv")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; VCFtools site and individual missingness are two commands; $scope_caveat"
  benchmark_pair "$tier" "HWE" "$dataset" "$population_source" \
    "$vf_base hardy $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.hwe")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --hardy --out $(shell_quote "$prefix/vcftools-hardy")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; exact p-value column is outside current output; $scope_caveat"
  benchmark_pair "$tier" "heterozygosity" "$dataset" "$population_source" \
    "$vf_base het $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.het")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --het --out $(shell_quote "$prefix/vcftools-het")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "site pi" "$dataset" "$population_source" \
    "$vf_base pi $(shell_quote "$dataset") -o $(shell_quote "$prefix/variantflow.sites.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --site-pi --out $(shell_quote "$prefix/vcftools-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; $scope_caveat"
  benchmark_pair "$tier" "window pi" "$dataset" "$population_source" \
    "$vf_base pi $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.windowed.pi")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --window-pi $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-window-pi")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE; $scope_caveat"
  benchmark_pair "$tier" "Tajima's D" "$dataset" "$population_source" \
    "$vf_base tajima-d $(shell_quote "$dataset") --window-size $(shell_quote "$WINDOW_SIZE") -o $(shell_quote "$prefix/variantflow.Tajima.D")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --TajimaD $(shell_quote "$WINDOW_SIZE") --out $(shell_quote "$prefix/vcftools-tajima-d")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; window size $WINDOW_SIZE; $scope_caveat"
  benchmark_pair "$tier" "LD" "$dataset" "$population_source" \
    "$vf_base ld $(shell_quote "$dataset") --max-distance $(shell_quote "$LD_WINDOW_BP") -o $(shell_quote "$prefix/variantflow.geno.ld")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --geno-r2 --ld-window-bp $(shell_quote "$LD_WINDOW_BP") --out $(shell_quote "$prefix/vcftools-ld")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; genotype dosage R2; $scope_caveat"
  benchmark_pair "$tier" "Weir-Cockerham Fst" "$dataset" "$population_source" \
    "$vf_base fst $(shell_quote "$dataset") --pop $(shell_quote "$pop1") --pop $(shell_quote "$pop2") --estimator weir-cockerham -o $(shell_quote "$prefix/variantflow.weir.fst")" \
    "$vcftools_base $input_flag $(shell_quote "$dataset") --weir-fst-pop $(shell_quote "$pop1") --weir-fst-pop $(shell_quote "$pop2") --out $(shell_quote "$prefix/vcftools-weir-fst")" \
    "matches VCFtools on supported diploid biallelic parity fixtures; two population files required; $scope_caveat"
}

for tier_limit in $PUBLIC_TIERS; do
  tier_staging_error="$OUT_DIR/public-cohort.${tier_limit}.staging.err"
  if ! tier_input="$(prepare_true_public_biallelic_dataset "$PUBLIC_INPUT" "$OUT_DIR/public-cohort.biallelic.${tier_limit}.vcf.gz" "$tier_limit" 2>"$tier_staging_error")"; then
    tier_blocker="$(tr '\n' ' ' <"$tier_staging_error" | sed 's/[[:space:]]*$//')"
    if [[ -z "$tier_blocker" ]]; then
      tier_blocker="blocked: failed to stage bounded true public biallelic cohort tier"
    fi
    append_pending_row "$tier_limit" "$tier_blocker"
    continue
  fi
  public_pop_files="$(public_population_files "$tier_input" "$tier_limit")"
  IFS=$'\t' read -r tier_pop1 tier_pop2 tier_population_metadata tier_population_source <<<"$public_pop_files"
  if [[ -z "$tier_pop1" || -z "$tier_pop2" || -z "$tier_population_metadata" || -z "$tier_population_source" ]]; then
    echo "Population metadata helper must return pop1, pop2, metadata, and source fields for public cohort $tier_limit." >&2
    exit 1
  fi
  run_tier "$tier_limit" "$tier_input" "$tier_pop1" "$tier_pop2" "$tier_population_source"
  python3 benchmark/check_vcftools_parity.py "$OUT_DIR/public cohort $tier_limit" >/tmp/v17-true-public-population-tier-check.txt 2>&1 || {
    cat /tmp/v17-true-public-population-tier-check.txt
    echo "True public benchmark outputs failed VCFtools parity checks for public cohort $tier_limit."
    exit 1
  }
done

cat >"$REPORT" <<EOF
# VariantFlow v1.7 True Public Population Evidence

Status: benchmark rows generated by \`make bench-vcftools-true-popgen\` for a
1000 Genomes / IGSR cohort staged from \`$PUBLIC_INPUT\` with official
population metadata from \`$PUBLIC_METADATA\`.

Runs: \`$RUNS\`; warmup: \`$WARMUP\`; public tiers: \`$PUBLIC_TIERS\`; groups:
\`$PUBLIC_GROUPS\`; group level: \`$PUBLIC_GROUP_LEVEL\`.

Public tier labels are requested staging limits; the actual record count column
records how many biallelic SNP records were available after
\`bcftools view -m2 -M2 -v snps\` streaming into bgzip. Population files come
from official IGSR metadata through \`$POPULATION_HELPER\`; the harness uses no
header-fallback population files.

Correctness gate: \`make vcftools-parity\` plus
\`benchmark/check_vcftools_parity.py\` on each measured tier output directory.
Measured rows report hyperfine runtime mean and resource metrics captured by
\`$RESOURCE_RUNNER\`, including VariantFlow/VCFtools peak RSS KB, CPU seconds,
and CPU-hour estimate. Each row includes exact commands and VCFtools version.

| tier | case | actual record count | sample count | population metadata source | runtime mean | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | exact VariantFlow command | exact VCFtools command | VCFtools version | correctness result | caveats |
|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|---|
$(cat "$ROWS_FILE")

Claim decision: no broad VCFtools replacement claim. Correctness-matched public
rows may support measured, scoped performance statements only for the staged
bounded biallelic 1000 Genomes / IGSR cohort in this report. This report does
not support a broad VCFtools replacement claim.
EOF

echo "VariantFlow v1.7 true public population evidence report written to $REPORT"
