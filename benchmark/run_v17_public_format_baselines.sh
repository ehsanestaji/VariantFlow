#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v17-public-format-baselines}"
REPORT="${VCF_FAST_V17_REPORT:-benchmark/reports/v17-public-format-baselines.md}"
PUBLIC_DATA="${VCF_FAST_FORMAT_VCF:-tests/output/public-data/NA12878.trio.hg19_multianno.vcf.gz}"
PUBLIC_SOURCE_URL="${VCF_FAST_FORMAT_VCF_URL:-https://sourceforge.net/projects/project123vcf/files/Benchmark_Data/NA12878.trio.hg19_multianno.vcf.gz/download}"
TIERS="${VCF_FAST_V17_TIERS:-10000 50000}"
FORMAT_EXPR='N_PASS(FORMAT/AD[1] > 10) >= 2'
BCFTOOLS_EXPR='N_PASS(FMT/AD[*:1]>10)>=2'

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

measure_peak_rss_kb() {
  local label="$1"
  shift
  if command -v /usr/bin/time >/dev/null 2>&1; then
    /usr/bin/time -l "$@" >"${OUT_DIR}/${label}.stdout" 2>"${OUT_DIR}/${label}.time" || return $?
    awk '/maximum resident set size/ {print $1}' "${OUT_DIR}/${label}.time" || true
  else
    "$@" >"${OUT_DIR}/${label}.stdout"
    echo "n/a"
  fi
}

real_seconds_from_time() {
  local time_file="$1"
  awk '/ real/ {print $1; exit}' "$time_file"
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
  stream_public_vcf | awk '
    /^##FORMAT=<ID=AD,/ { ad = 1 }
    /^##FORMAT=<ID=DP,/ { dp = 1 }
    END { if (!(ad && dp)) exit 1 }
  '
}

write_header() {
  cat >"$REPORT" <<EOF
# v1.7 Public FORMAT And Optional Baselines

This report tracks public FORMAT-heavy and ecosystem baseline evidence. Full
runs stay local and reproducible; CI should use smoke tiers only.

Dataset target: FORMAT-rich public trio/cohort VCF. Default target is the
SourceForge 123VCF NA12878 trio benchmark because it declares FORMAT/AD and
FORMAT/DP; override with \`VCF_FAST_FORMAT_VCF\` for larger public cohorts.

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|
EOF
}

append_optional_baselines() {
  cat >>"$REPORT" <<EOF

## Optional baselines

- VCFtools: enabled only with \`VCF_FAST_ENABLE_VCFTOOLS=1\`.
- GATK SelectVariants / VariantFiltration: enabled only with \`VCF_FAST_ENABLE_GATK=1\`.
- Polars: enabled only with \`VCF_FAST_ENABLE_POLARS=1\`.
- PyArrow: enabled only with \`VCF_FAST_ENABLE_PYARROW=1\`.

Optional baseline rows remain \`not yet proven\` until correctness and runtime
are recorded.
EOF
}

if [[ ! -f "$PUBLIC_DATA" ]]; then
  write_header
  echo "| public FORMAT-heavy | $PUBLIC_DATA | n/a | n/a | n/a | missing public data | n/a | n/a | not yet proven | run benchmark/download_public_data.sh format-trio |" >>"$REPORT"
  append_optional_baselines
  exit 0
fi

if ! public_vcf_has_required_format; then
  write_header
  echo "| public FORMAT-heavy | $PUBLIC_DATA | n/a | n/a | n/a | missing FORMAT/AD or FORMAT/DP declaration | n/a | n/a | not yet proven | choose a FORMAT-rich public VCF with VCF_FAST_FORMAT_VCF; IGSR high-coverage chr22 is GT-only |" >>"$REPORT"
  append_optional_baselines
  exit 0
fi

write_header
cargo build --release

for tier in $TIERS; do
  subset="${OUT_DIR}/format-public-${tier}.vcf.gz"
  fast_out="${OUT_DIR}/variantflow-format-${tier}.vcf"
  bcftools_out="${OUT_DIR}/bcftools-format-${tier}.vcf"
  diff_out="${OUT_DIR}/equivalence-format-${tier}.diff"

  stream_public_vcf | awk -v limit="$tier" '
    BEGIN { records = 0 }
    /^#/ { print; next }
    records < limit { print; records++ }
  ' | bgzip -c >"$subset"
  tabix -f -p vcf "$subset"

  fast_cmd=(./target/release/variantflow filter "$subset" --where "$FORMAT_EXPR" -o "$fast_out")
  bcftools_cmd=(bcftools filter -i "$BCFTOOLS_EXPR" "$subset" -o "$bcftools_out")

  fast_label="variantflow-format-${tier}"
  bcftools_label="bcftools-format-${tier}"
  fast_rss=$(measure_peak_rss_kb "$fast_label" "${fast_cmd[@]}")
  bcftools_rss=$(measure_peak_rss_kb "$bcftools_label" "${bcftools_cmd[@]}")
  fast_seconds=$(real_seconds_from_time "${OUT_DIR}/${fast_label}.time")
  bcftools_seconds=$(real_seconds_from_time "${OUT_DIR}/${bcftools_label}.time")
  speedup=$(speedup_ratio "$fast_seconds" "$bcftools_seconds")
  diff <(grep -v '^#' "$fast_out" | cut -f1-5) <(grep -v '^#' "$bcftools_out" | cut -f1-5) >"$diff_out" || true

  if [[ -s "$diff_out" ]]; then
    correctness="not matched"
    claim="no performance claim"
  elif python3 - "$fast_seconds" "$bcftools_seconds" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) < float(sys.argv[2]) else 1)
PY
  then
    correctness="matched core records"
    claim="measured faster on this public FORMAT-rich tier"
  else
    correctness="matched core records"
    claim="correctness matched; optimization needed before claiming speed win"
  fi

  echo "| public FORMAT-heavy | $PUBLIC_DATA | $tier | \`$(shell_command "${fast_cmd[@]}")\` | \`$(shell_command "${bcftools_cmd[@]}")\` | $correctness | VariantFlow ${fast_seconds}s; bcftools ${bcftools_seconds}s; speedup ${speedup} | VariantFlow ${fast_rss}; bcftools ${bcftools_rss} | $claim | FORMAT-rich public trio/cohort source: $PUBLIC_SOURCE_URL; expression uses $FORMAT_EXPR; compare against bcftools filter |" >>"$REPORT"
done

if [[ "${VCF_FAST_ENABLE_VCFTOOLS:-0}" = "1" ]]; then
  echo "| VCFtools optional baseline | $PUBLIC_DATA | n/a | n/a | vcftools optional command | not yet proven | n/a | n/a | not yet proven | VCFtools installed baseline requested |" >>"$REPORT"
fi

if [[ "${VCF_FAST_ENABLE_GATK:-0}" = "1" ]]; then
  echo "| GATK optional baseline | $PUBLIC_DATA | n/a | n/a | gatk VariantFiltration optional command | not yet proven | n/a | n/a | not yet proven | GATK installed baseline requested |" >>"$REPORT"
fi

if [[ "${VCF_FAST_ENABLE_POLARS:-0}" = "1" ]]; then
  echo "| Polars optional baseline | Parquet export | n/a | variantflow convert --to parquet | polars query optional command | not yet proven | n/a | n/a | not yet proven | Polars installed baseline requested |" >>"$REPORT"
fi

if [[ "${VCF_FAST_ENABLE_PYARROW:-0}" = "1" ]]; then
  echo "| PyArrow optional baseline | Parquet export | n/a | variantflow convert --to parquet | pyarrow query optional command | not yet proven | n/a | n/a | not yet proven | PyArrow installed baseline requested |" >>"$REPORT"
fi

append_optional_baselines
