#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v17-public-format-baselines}"
REPORT="${VCF_FAST_V17_REPORT:-benchmark/reports/v17-public-format-baselines.md}"
PUBLIC_DATA="${VCF_FAST_IGSR_VCF:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
REGION="${VCF_FAST_V17_REGION:-chr22:1-20000000}"
TIERS="${VCF_FAST_V17_TIERS:-100000 1000000}"
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

write_header() {
  cat >"$REPORT" <<EOF
# v1.7 Public FORMAT And Optional Baselines

This report is the scaffold for public FORMAT-heavy and ecosystem baseline
evidence. Full runs stay local and reproducible; CI should use smoke tiers only.

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

Rows remain \`not yet proven\` until correctness and runtime are recorded.
EOF
}

if [[ ! -f "$PUBLIC_DATA" ]]; then
  write_header
  echo "| public FORMAT-heavy | $PUBLIC_DATA | n/a | n/a | n/a | missing public data | n/a | n/a | not yet proven | run benchmark/download_public_data.sh igsr-chr22 |" >>"$REPORT"
  append_optional_baselines
  exit 0
fi

write_header

for tier in $TIERS; do
  subset="${OUT_DIR}/igsr-format-${tier}.vcf.gz"
  fast_out="${OUT_DIR}/variantflow-format-${tier}.vcf"
  bcftools_out="${OUT_DIR}/bcftools-format-${tier}.vcf"
  diff_out="${OUT_DIR}/equivalence-format-${tier}.diff"

  {
    bcftools view -h "$PUBLIC_DATA"
    bcftools view -H -r "$REGION" "$PUBLIC_DATA" | awk -v limit="$tier" 'NR <= limit'
  } | bgzip -c >"$subset"
  tabix -f -p vcf "$subset"

  fast_cmd=(variantflow filter "$subset" --where "$FORMAT_EXPR" -o "$fast_out")
  bcftools_cmd=(bcftools filter -i "$BCFTOOLS_EXPR" "$subset" -o "$bcftools_out")

  fast_rss=$(measure_peak_rss_kb "variantflow-format-${tier}" "${fast_cmd[@]}")
  bcftools_rss=$(measure_peak_rss_kb "bcftools-format-${tier}" "${bcftools_cmd[@]}")
  diff <(grep -v '^#' "$fast_out" | cut -f1-5) <(grep -v '^#' "$bcftools_out" | cut -f1-5) >"$diff_out" || true

  if [[ -s "$diff_out" ]]; then
    correctness="not matched"
    claim="no performance claim"
  else
    correctness="matched core records"
    claim="measured row; inspect runtime before claiming win"
  fi

  echo "| public FORMAT-heavy | $PUBLIC_DATA | $tier | \`${fast_cmd[*]}\` | \`${bcftools_cmd[*]}\` | $correctness | see local timing files | VariantFlow ${fast_rss}; bcftools ${bcftools_rss} | $claim | public FORMAT expression uses $FORMAT_EXPR; compare against bcftools filter |" >>"$REPORT"
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
