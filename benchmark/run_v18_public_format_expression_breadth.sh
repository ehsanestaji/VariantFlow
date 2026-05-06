#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v18-public-format-expression-breadth}"
REPORT="${VCF_FAST_V18_REPORT:-benchmark/reports/v18-public-format-expression-breadth.md}"
FORMAT_COHORT_VCF="${VCF_FAST_FORMAT_COHORT_VCF:-tests/output/public-data/19.filtered_intersect.vcf.gz}"
FORMAT_COHORT_URL="https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz"
FORMAT_COHORT_ENA_ACCESSION="ERZ324584"
FORMAT_COHORT_BYTES="2213677122"
FORMAT_COHORT_RECORDS="1097167"
FORMAT_COHORT_SAMPLES="453"
FORMAT_COHORT_MD5="9dabe9929a8923e62c8808d6fbf15314"
PUBLIC_DATA="${VCF_FAST_FORMAT_VCF:-$FORMAT_COHORT_VCF}"
PUBLIC_SOURCE_URL="${VCF_FAST_FORMAT_VCF_URL:-$FORMAT_COHORT_URL}"
TIERS="${VCF_FAST_V18_TIERS:-1000000 full}"
RUNS="${VCF_FAST_V18_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V18_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
HEAVY_OUTPUT_RECORDS="${VCF_FAST_V18_HEAVY_OUTPUT_RECORDS:-500000}"
BCFTOOLS_VERSION="$(bcftools --version 2>/dev/null | head -1 || echo "bcftools unavailable")"

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

measure_peak_rss_kb() {
  local label="$1"
  shift
  if command -v /usr/bin/time >/dev/null 2>&1; then
    if /usr/bin/time -v true >/dev/null 2>&1; then
      /usr/bin/time -v -o "${OUT_DIR}/${label}.time" "$@" >"${OUT_DIR}/${label}.stdout" 2>"${OUT_DIR}/${label}.stderr" || return $?
      awk -F: '/Maximum resident set size/ {gsub(/ /, "", $2); print $2}' "${OUT_DIR}/${label}.time" || true
    else
      /usr/bin/time -l "$@" >"${OUT_DIR}/${label}.stdout" 2>"${OUT_DIR}/${label}.time" || return $?
      awk '/maximum resident set size/ {print $1}' "${OUT_DIR}/${label}.time" || true
    fi
  else
    "$@" >"${OUT_DIR}/${label}.stdout"
    echo "n/a"
  fi
}

speedup_ratio() {
  local fast_seconds="$1"
  local competitor_seconds="$2"
  python3 - "$fast_seconds" "$competitor_seconds" <<'PY'
import sys

fast = float(sys.argv[1])
competitor = float(sys.argv[2])
print("n/a" if fast <= 0 else f"{competitor / fast:.2f}x")
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
stddev = result.get("stddev")
if stddev is None:
    stddev = 0.0
print(f'{result["mean"]:.6f} {stddev:.6f}')
PY
  else
    local start_seconds end_seconds
    start_seconds="$(python3 - <<'PY'
import time
print(f"{time.perf_counter():.9f}")
PY
)"
    eval "$command_text" >"${OUT_DIR}/${label}.runtime.stdout" 2>"${OUT_DIR}/${label}.runtime.stderr"
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

shell_command() {
  printf "%q " "$@"
}

stream_public_vcf() {
  case "$PUBLIC_DATA" in
    *.gz|*.bgz)
      gzip -cd "$PUBLIC_DATA"
      ;;
    *)
      cat "$PUBLIC_DATA"
      ;;
  esac
}

public_vcf_has_required_format() {
  set +e
  stream_public_vcf | awk '
    /^##FORMAT=<ID=AD,/ { ad = 1 }
    /^##FORMAT=<ID=DP,/ { dp = 1 }
    /^##FORMAT=<ID=GQ,/ { gq = 1 }
    /^#CHROM/ { exit(ad && dp && gq ? 0 : 1) }
    END { if (!(ad && dp && gq)) exit 1 }
  '
  local statuses=("${PIPESTATUS[@]}")
  set -e
  [[ "${statuses[1]}" -eq 0 ]]
}

sample_count() {
  local vcf="$1"
  bcftools query -l "$vcf" | wc -l | tr -d ' '
}

selected_sample() {
  local vcf="$1"
  if [[ -n "${VCF_FAST_V18_SAMPLE:-}" ]]; then
    echo "$VCF_FAST_V18_SAMPLE"
  else
    bcftools query -l "$vcf" | sed -n '2p'
  fi
}

record_count() {
  local vcf="$1"
  local count
  count="$(bcftools index -n "$vcf" 2>/dev/null || true)"
  if [[ -n "$count" ]]; then
    echo "$count"
  else
    bcftools view -H "$vcf" | wc -l | tr -d ' '
  fi
}

build_bounded_subset() {
  local tier="$1"
  local output="$2"
  set +e
  stream_public_vcf | awk -v limit="$tier" '
    BEGIN { records = 0 }
    /^#/ { print; next }
    records < limit { print; records++ }
    records >= limit { exit }
  ' | bgzip -c >"$output"
  local statuses=("${PIPESTATUS[@]}")
  set -e
  if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
    return 1
  fi
  tabix -f -p vcf "$output"
}

tier_label() {
  local tier="$1"
  case "$tier" in
    full|all|full-chromosome)
      echo "full"
      ;;
    *)
      echo "$tier"
      ;;
  esac
}

prepare_tier_dataset() {
  local tier="$1"
  local output="$2"
  case "$tier" in
    full|all|full-chromosome)
      echo "$PUBLIC_DATA"
      ;;
    *)
      build_bounded_subset "$tier" "$output"
      echo "$output"
      ;;
  esac
}

heavy_output_mode() {
  local tier="$1"
  local records="$2"
  case "$tier" in
    full|all|full-chromosome)
      return 0
      ;;
  esac
  [[ "$records" =~ ^[0-9]+$ && "$records" -ge "$HEAVY_OUTPUT_RECORDS" ]]
}

write_core_records() {
  awk 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5 }'
}

run_fast_core() {
  local subset="$1"
  local expr="$2"
  local sample="$3"
  local output="$4"
  if [[ -n "$sample" ]]; then
    ./target/release/variantflow filter "$subset" --sample "$sample" --where "$expr" -o /dev/stdout \
      | write_core_records >"$output"
  else
    ./target/release/variantflow filter "$subset" --where "$expr" -o /dev/stdout \
      | write_core_records >"$output"
  fi
}

run_bcftools_core() {
  local subset="$1"
  local expr="$2"
  local sample="$3"
  local output="$4"
  if [[ -n "$sample" ]]; then
    bcftools view -s "$sample" "$subset" -Ou \
      | bcftools filter -Ov -i "$expr" -o /dev/stdout \
      | write_core_records >"$output"
  else
    bcftools filter -Ov -i "$expr" "$subset" -o /dev/stdout \
      | write_core_records >"$output"
  fi
}

bcftools_command_text() {
  local subset="$1"
  local expr="$2"
  local sample="$3"
  if [[ -n "$sample" ]]; then
    printf "bcftools view -s %q %q -Ou | bcftools filter -Ov -i %q -o /dev/stdout" "$sample" "$subset" "$expr"
  else
    shell_command bcftools filter -Ov -i "$expr" "$subset" -o /dev/stdout
  fi
}

bcftools_timed_command() {
  local subset="$1"
  local expr="$2"
  local sample="$3"
  if [[ -n "$sample" ]]; then
    printf "bcftools view -s %q %q -Ou | bcftools filter -Ov -i %q -o /dev/null" "$sample" "$subset" "$expr"
  else
    shell_command bcftools filter -Ov -i "$expr" "$subset" -o /dev/null
  fi
}

bcftools_rss() {
  local label="$1"
  local subset="$2"
  local expr="$3"
  local sample="$4"
  if [[ -n "$sample" ]]; then
    measure_peak_rss_kb "$label" bash -o pipefail -c 'bcftools view -s "$1" "$2" -Ou | bcftools filter -Ov -i "$3" -o /dev/null' _ "$sample" "$subset" "$expr"
  else
    measure_peak_rss_kb "$label" bcftools filter -Ov -i "$expr" "$subset" -o /dev/null
  fi
}

write_header() {
  cat >"$REPORT" <<EOF
# v1.8 Public FORMAT Expression Breadth

This report expands public FORMAT-rich evidence from one aggregate AD predicate
to a broader expression set on the same cached ENA Ovis aries cohort. The
default target is ENA \`$FORMAT_COHORT_ENA_ACCESSION\`,
\`19.filtered_intersect.vcf.gz\`: a 453-sample chromosome 19 VCF with declared
\`FORMAT/AD\`, \`FORMAT/DP\`, and \`FORMAT/GQ\`, \`$FORMAT_COHORT_RECORDS\`
indexed records, \`$FORMAT_COHORT_BYTES\` bytes, and MD5
\`$FORMAT_COHORT_MD5\`.

Every row compares VariantFlow against \`bcftools filter\` and claims speed only
when filtered core records match. Heavy-output mode is enabled by default for
tiers at or above \`$HEAVY_OUTPUT_RECORDS\` records and for \`full\` tiers:
correctness streams \`/dev/stdout\` into core records only, while timed runs
write to \`/dev/null\`.

Repeated timing uses \`hyperfine\` when available
(\`VCF_FAST_V18_RUNS=$RUNS\`, \`VCF_FAST_V18_WARMUP=$WARMUP\`). Peak RSS is
reported from GNU \`/usr/bin/time -v\` on Linux or BSD \`/usr/bin/time -l\` on
macOS. Competitor version for this run: \`$BCFTOOLS_VERSION\`.

Planned expressions:

- \`ANY(FORMAT/DP > 20)\`
- \`ALL(FORMAT/GQ >= 30)\`
- \`N_PASS(FORMAT/AD[1] > 10) >= 10\`
- selected-sample \`FORMAT/DP > 20\`
- \`QUAL > 30 && ANY(FORMAT/DP > 20)\`

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | variants/sec | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|---|
EOF
}

append_not_yet_proven_rows() {
  cat >>"$REPORT" <<EOF
| ANY(FORMAT/DP > 20) | $PUBLIC_DATA | n/a | n/a | \`bcftools filter -i 'N_PASS(FMT/DP[*]>20)>0'\` | not yet proven | n/a | n/a | n/a | not yet proven | benchmark did not run |
| ALL(FORMAT/GQ >= 30) | $PUBLIC_DATA | n/a | n/a | \`bcftools filter -i 'N_PASS(FMT/GQ[*]>=30)==<sample-count>'\` | not yet proven | n/a | n/a | n/a | not yet proven | benchmark did not run |
| N_PASS(FORMAT/AD[1] > 10) >= 10 | $PUBLIC_DATA | n/a | n/a | \`bcftools filter -i 'N_PASS(FMT/AD[*:1]>10)>=10'\` | not yet proven | n/a | n/a | n/a | not yet proven | benchmark did not run |
| selected-sample FORMAT/DP > 20 | $PUBLIC_DATA | n/a | n/a | \`bcftools view -s <sample> ... | bcftools filter -i 'FMT/DP[0]>20'\` | not yet proven | n/a | n/a | n/a | not yet proven | benchmark did not run |
| QUAL > 30 && ANY(FORMAT/DP > 20) | $PUBLIC_DATA | n/a | n/a | \`bcftools filter -i 'QUAL>30 && N_PASS(FMT/DP[*]>20)>0'\` | not yet proven | n/a | n/a | n/a | not yet proven | benchmark did not run |
EOF
}

if [[ ! -f "$PUBLIC_DATA" ]]; then
  write_header
  append_not_yet_proven_rows
  exit 0
fi

if ! public_vcf_has_required_format; then
  write_header
  append_not_yet_proven_rows
  echo "" >>"$REPORT"
  echo "Required FORMAT metadata was not found; use \`benchmark/download_public_data.sh format-ovis453\` or set \`VCF_FAST_FORMAT_VCF\` to a cohort declaring FORMAT/AD, FORMAT/DP, and FORMAT/GQ." >>"$REPORT"
  exit 0
fi

write_header
cargo build --release

for tier in $TIERS; do
  label="$(tier_label "$tier")"
  subset_candidate="${OUT_DIR}/format-expression-${label}.vcf.gz"
  subset="$(prepare_tier_dataset "$tier" "$subset_candidate")"
  actual_records="$(record_count "$subset")"
  samples="$(sample_count "$subset")"
  selected="$(selected_sample "$subset")"
  output_policy="smoke/core-stream output"
  if heavy_output_mode "$tier" "$actual_records"; then
    output_policy="heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs"
  fi

  cases=(
    "ANY(FORMAT/DP > 20)|ANY(FORMAT/DP > 20)|N_PASS(FMT/DP[*]>20)>0|"
    "ALL(FORMAT/GQ >= 30)|ALL(FORMAT/GQ >= 30)|N_PASS(FMT/GQ[*]>=30)==${samples}|"
    "N_PASS(FORMAT/AD[1] > 10) >= 10|N_PASS(FORMAT/AD[1] > 10) >= 10|N_PASS(FMT/AD[*:1]>10)>=10|"
    "selected-sample FORMAT/DP > 20|FORMAT/DP > 20|FMT/DP[0]>20|$selected"
    "QUAL > 30 && ANY(FORMAT/DP > 20)|QUAL > 30 && ANY(FORMAT/DP > 20)|QUAL>30 && N_PASS(FMT/DP[*]>20)>0|"
  )

  for case_def in "${cases[@]}"; do
    IFS='|' read -r case_name fast_expr bcftools_expr sample <<<"$case_def"
    slug="$(echo "${case_name}-${label}" | tr '[:upper:] /()[]>=' '[:lower:]----------' | tr -cd '[:alnum:]-')"
    fast_core="${OUT_DIR}/${slug}.variantflow.core.tsv"
    bcftools_core="${OUT_DIR}/${slug}.bcftools.core.tsv"
    diff_out="${OUT_DIR}/${slug}.diff"
    fast_label="${slug}.variantflow"
    bcftools_label="${slug}.bcftools"

    run_fast_core "$subset" "$fast_expr" "$sample" "$fast_core"
    run_bcftools_core "$subset" "$bcftools_expr" "$sample" "$bcftools_core"
    diff "$fast_core" "$bcftools_core" >"$diff_out" || true

    fast_cmd=(./target/release/variantflow filter "$subset" --where "$fast_expr" -o /dev/stdout)
    fast_timed=(./target/release/variantflow filter "$subset" --where "$fast_expr" -o /dev/null)
    fast_rss_cmd=(./target/release/variantflow filter "$subset" --where "$fast_expr" -o /dev/null)
    if [[ -n "$sample" ]]; then
      fast_cmd=(./target/release/variantflow filter "$subset" --sample "$sample" --where "$fast_expr" -o /dev/stdout)
      fast_timed=(./target/release/variantflow filter "$subset" --sample "$sample" --where "$fast_expr" -o /dev/null)
      fast_rss_cmd=(./target/release/variantflow filter "$subset" --sample "$sample" --where "$fast_expr" -o /dev/null)
    fi

    fast_rss="$(measure_peak_rss_kb "$fast_label" "${fast_rss_cmd[@]}")"
    bcftools_rss_value="$(bcftools_rss "$bcftools_label" "$subset" "$bcftools_expr" "$sample")"
    read -r fast_seconds fast_stddev <<<"$(runtime_mean_stddev "$fast_label" "$(shell_command "${fast_timed[@]}")")"
    read -r bcftools_seconds bcftools_stddev <<<"$(runtime_mean_stddev "$bcftools_label" "$(bcftools_timed_command "$subset" "$bcftools_expr" "$sample")")"

    speedup="$(speedup_ratio "$fast_seconds" "$bcftools_seconds")"
    fast_vps="$(variants_per_second "$actual_records" "$fast_seconds")"
    bcftools_vps="$(variants_per_second "$actual_records" "$bcftools_seconds")"
    if [[ -s "$diff_out" ]]; then
      correctness="not matched"
      claim="no speed claim; fix correctness first"
    elif python3 - "$fast_seconds" "$bcftools_seconds" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) < float(sys.argv[2]) else 1)
PY
    then
      correctness="matched core records"
      claim="measured faster on this public FORMAT-rich expression tier"
    else
      correctness="matched core records"
      claim="correctness matched; optimization needed before speed claim"
    fi

    echo "| $case_name | $PUBLIC_DATA | $tier requested / $actual_records actual | \`$(shell_command "${fast_cmd[@]}")\` | \`$(bcftools_command_text "$subset" "$bcftools_expr" "$sample")\` | $correctness | VariantFlow ${fast_seconds}s +/- ${fast_stddev}s; bcftools ${bcftools_seconds}s +/- ${bcftools_stddev}s; speedup ${speedup} | VariantFlow ${fast_vps}; bcftools ${bcftools_vps} | VariantFlow ${fast_rss}; bcftools ${bcftools_rss_value} | $claim | source=$PUBLIC_SOURCE_URL; accession=$FORMAT_COHORT_ENA_ACCESSION; samples=$samples; selected_sample=${sample:-n/a}; output policy: $output_policy |" >>"$REPORT"
  done
done
