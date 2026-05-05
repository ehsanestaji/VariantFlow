#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_BENCH_REPORT:-$OUT_DIR/benchmark-report.md}"
MODE="${VCF_FAST_BENCH_MODE:-synthetic}"
SIZES="${VCF_FAST_BENCH_SIZES:-10000 100000}"
PUBLIC_RECORD_TIERS="${VCF_FAST_PUBLIC_RECORD_TIERS:-10000 100000 1000000}"
PUBLIC_SOURCE_KIND="${VCF_FAST_PUBLIC_SOURCE:-giab-hg002}"
PUBLIC_RECORDS="${VCF_FAST_PUBLIC_RECORDS:-10000}"
PUBLIC_REGION="${VCF_FAST_PUBLIC_REGION:-chr22:1-20000000}"
HEAVY_MAX_PLAIN_BYTES="${VCF_FAST_HEAVY_MAX_PLAIN_BYTES:-1073741824}"
HEAVY_REGION="${VCF_FAST_HEAVY_REGION:-$PUBLIC_REGION}"
COMPAT_REGION="${VCF_FAST_COMPAT_REGION:-22:1-20000000}"
STRESS_INFO_FIELDS="${VCF_FAST_STRESS_INFO_FIELDS:-40}"
STRESS_SAMPLES="${VCF_FAST_STRESS_SAMPLES:-16}"
CASES=(
  "QUAL plain|plain|QUAL > 30|QUAL>30"
  "DP plain|plain|DP > 40|INFO/DP>40"
  "AF plain|plain|AF > 0.2|INFO/AF>0.2"
  "QUAL gzip input|gzip|QUAL > 30|QUAL>30"
  "Convert TSV|plain|convert-tsv|query-tsv"
  "Stats JSON|plain|stats-json|stats"
)

tool_version() {
  local tool="$1"
  if command -v "$tool" >/dev/null 2>&1; then
    "$tool" --version 2>&1 | head -n 1
  else
    echo "unavailable"
  fi
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

extract_core_records() {
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$1"
}

compare_stats_counts() {
  local fast_json="$1"
  local bcftools_stats="$2"
  local diff_output="$3"
  local fast_variants
  local bcftools_records

  fast_variants="$(python3 - "$fast_json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["variants"])
PY
)"
  bcftools_records="$(awk -F '\t' '$1 == "SN" && $3 == "number of records:" { print $4; found = 1 } END { exit found ? 0 : 1 }' "$bcftools_stats")"

  if [[ "$fast_variants" == "$bcftools_records" ]]; then
    : >"$diff_output"
  else
    {
      echo "vcf-fast variants: $fast_variants"
      echo "bcftools records: $bcftools_records"
    } >"$diff_output"
    return 1
  fi
}

slugify() {
  echo "$1" | tr '[:upper:] ' '[:lower:]-' | tr -cd '[:alnum:]-'
}

markdown_cell() {
  local value="$1"
  printf '%s\n' "$value" | sed 's/|/\&#124;/g'
}

file_size_bytes() {
  wc -c <"$1" | tr -d ' '
}

assert_plain_artifact_under_cap() {
  local path="$1"
  local max_bytes="$2"
  local actual
  actual="$(file_size_bytes "$path")"
  if (( actual > max_bytes )); then
    echo "deferred: plain artifact cap exceeded for $path (${actual} > ${max_bytes})" >&2
    rm -f "$path"
    return 77
  fi
}

predicate_check() {
  local output="$1"
  local expression="$2"

  case "$expression" in
    "QUAL > 30")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } $6 <= 30 { ok = 0; print "record failed QUAL > 30: " $0 > "/dev/stderr" } END { exit ok ? 0 : 1 }' "$output"
      ;;
    "DP > 40")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } { split($8, fields, ";"); dp = ""; for (i in fields) { if (fields[i] ~ /^DP=/) { sub(/^DP=/, "", fields[i]); dp = fields[i] } } if (dp <= 40) { ok = 0; print "record failed DP > 40: " $0 > "/dev/stderr" } } END { exit ok ? 0 : 1 }' "$output"
      ;;
    "AF > 0.2")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } { split($8, fields, ";"); af = ""; for (i in fields) { if (fields[i] ~ /^AF=/) { sub(/^AF=/, "", fields[i]); af = fields[i] } } if (af <= 0.2) { ok = 0; print "record failed AF > 0.2: " $0 > "/dev/stderr" } } END { exit ok ? 0 : 1 }' "$output"
      ;;
    "FORMAT/DP > 20")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } { split($9, keys, ":"); split($10, values, ":"); dp = ""; for (i in keys) { if (keys[i] == "DP") { dp = values[i] } } if (dp <= 20) { ok = 0; print "record failed FORMAT/DP > 20: " $0 > "/dev/stderr" } } END { exit ok ? 0 : 1 }' "$output"
      ;;
    "FORMAT/GQ >= 30")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } { split($9, keys, ":"); split($10, values, ":"); gq = ""; for (i in keys) { if (keys[i] == "GQ") { gq = values[i] } } if (gq < 30) { ok = 0; print "record failed FORMAT/GQ >= 30: " $0 > "/dev/stderr" } } END { exit ok ? 0 : 1 }' "$output"
      ;;
    "FORMAT/GT == \"0/1\"")
      awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } { split($9, keys, ":"); split($10, values, ":"); gt = ""; for (i in keys) { if (keys[i] == "GT") { gt = values[i] } } if (gt != "0/1") { ok = 0; print "record failed FORMAT/GT == \"0/1\": " $0 > "/dev/stderr" } } END { exit ok ? 0 : 1 }' "$output"
      ;;
    *)
      echo "no predicate check defined for $expression" >&2
      return 1
      ;;
  esac
}

build_public_small_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"

  require_tool bcftools
  {
    bcftools view -h "$source"
    bcftools view -H "$source" | awk -v limit="$records" 'NR <= limit'
  } >"$output"
}

build_public_region_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"
  local region="$4"

  require_tool bcftools
  require_tool tabix
  {
    bcftools view -h "$source"
    bcftools view -H -r "$region" "$source" | awk -v limit="$records" 'NR <= limit'
  } >"$output"

  if ! awk 'BEGIN { found = 0 } !/^#/ { found = 1 } END { exit found ? 0 : 1 }' "$output"; then
    echo "region $region produced no records from $source; set VCF_FAST_PUBLIC_REGION to a matching indexed region" >&2
    exit 2
  fi
}

build_public_heavy_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"
  local region="$4"

  require_tool bcftools
  require_tool bgzip
  require_tool tabix

  local temp_plain="${output%.gz}.plain.tmp.vcf"

  if ! {
    bcftools view -h "$source"
    bcftools view -H -r "$region" "$source" | awk -v limit="$records" 'NR <= limit'
  } >"$temp_plain"; then
    echo "failed to stage public-heavy records from $source region $region" >&2
    rm -f "$temp_plain"
    return 2
  fi

  if ! awk 'BEGIN { found = 0 } !/^#/ { found = 1 } END { exit found ? 0 : 1 }' "$temp_plain"; then
    echo "region $region produced no records from $source; set VCF_FAST_HEAVY_REGION to a matching indexed region" >&2
    rm -f "$temp_plain"
    return 2
  fi

  if ! assert_plain_artifact_under_cap "$temp_plain" "$HEAVY_MAX_PLAIN_BYTES"; then
    return 77
  fi

  if ! bgzip -c "$temp_plain" >"$output"; then
    echo "failed to bgzip public-heavy dataset $output" >&2
    rm -f "$temp_plain"
    return 2
  fi
  if ! tabix -f -p vcf "$output"; then
    echo "failed to index public-heavy dataset $output" >&2
    rm -f "$temp_plain"
    return 2
  fi
  rm -f "$temp_plain"
}

sort_vcf_for_indexing() {
  local input="$1"
  local output="$2"

  {
    awk '/^#/ { print }' "$input"
    awk '!/^#/ { print }' "$input" | sort -t "$(printf '\t')" -k1,1V -k2,2n
  } >"$output"
}

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for VCF_FAST_BENCH_MODE=$MODE" >&2
    exit 2
  fi
}

configure_inputs() {
  case "$MODE" in
    synthetic)
      echo "synthetic generated data|"
      ;;
    public-small)
      local giab="tests/output/public-data/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz"
      if [[ ! -s "$giab" ]]; then
        echo "missing $giab; run benchmark/download_public_data.sh giab-hg002 first" >&2
        exit 2
      fi
      echo "GIAB HG002 v4.2.1 requested record subsets|$giab"
      ;;
    public-region)
      local igsr="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
      if [[ ! -s "$igsr" ]]; then
        echo "missing $igsr; run benchmark/download_public_data.sh igsr-chr22 first" >&2
        exit 2
      fi
      echo "1000 Genomes high-coverage chr22 region ${PUBLIC_REGION}, requested record subsets|$igsr"
      ;;
    public-heavy)
      local igsr="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
      if [[ ! -s "$igsr" ]]; then
        echo "missing $igsr; run benchmark/download_public_data.sh igsr-chr22 first" >&2
        exit 2
      fi
      echo "1000 Genomes high-coverage chr22 heavy compressed/indexed tiers ${HEAVY_REGION}|$igsr"
      ;;
    public-whole)
      local source=""
      local source_label=""
      case "$PUBLIC_SOURCE_KIND" in
        giab-hg002)
          source="tests/output/public-data/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz"
          source_label="GIAB HG002 v4.2.1 whole-file record-count tiers"
          ;;
        igsr-chr22)
          source="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
          source_label="1000 Genomes high-coverage chr22 whole-file record-count tiers"
          ;;
        *)
          echo "unsupported VCF_FAST_PUBLIC_SOURCE=$PUBLIC_SOURCE_KIND; expected giab-hg002 or igsr-chr22" >&2
          exit 2
          ;;
      esac
      if [[ ! -s "$source" ]]; then
        echo "missing $source; run benchmark/download_public_data.sh $PUBLIC_SOURCE_KIND first" >&2
        exit 2
      fi
      echo "$source_label|$source"
      ;;
    public-region-repeated)
      local igsr="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
      if [[ ! -s "$igsr" ]]; then
        echo "missing $igsr; run benchmark/download_public_data.sh igsr-chr22 first" >&2
        exit 2
      fi
      echo "1000 Genomes high-coverage chr22 repeated indexed region ${PUBLIC_REGION}|$igsr"
      ;;
    stress)
      echo "stress synthetic data|"
      ;;
    compatibility)
      echo "compatibility synthetic BCF/BGZF/indexed data|"
      ;;
    *)
      echo "unsupported VCF_FAST_BENCH_MODE=$MODE; expected synthetic, stress, public-small, public-region, public-heavy, public-whole, public-region-repeated, or compatibility" >&2
      exit 2
      ;;
  esac
}

mkdir -p "$OUT_DIR" "$DATA_DIR"
mkdir -p "$(dirname "$REPORT")"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi
if [[ "$MODE" == "compatibility" || "$MODE" == "public-region-repeated" || "$MODE" == "public-heavy" ]]; then
  cargo build --release --features htslib-static
else
  cargo build --release
fi
IFS='|' read -r DATASET_SOURCE PUBLIC_SOURCE < <(configure_inputs)
DATASET_SHAPE="records only"

if [[ "$MODE" == "stress" ]]; then
  DATASET_SHAPE="stress INFO fields=${STRESS_INFO_FIELDS}, samples=${STRESS_SAMPLES}, FORMAT=GT:DP:GQ:AD"
elif [[ "$MODE" == "compatibility" ]]; then
  DATASET_SHAPE="synthetic VCF plus BCF, BGZF, tabix index, and region ${COMPAT_REGION}"
elif [[ "$MODE" == "public-heavy" ]]; then
  DATASET_SHAPE="bounded BGZF/tabix public region ${HEAVY_REGION}, max plain staging bytes=${HEAVY_MAX_PLAIN_BYTES}"
fi

if [[ "$MODE" == "public-small" || "$MODE" == "public-whole" || "$MODE" == "public-region" || "$MODE" == "public-region-repeated" ]]; then
  CASES=(
    "QUAL plain|plain|QUAL > 30|QUAL>30"
    "QUAL gzip input|gzip|QUAL > 30|QUAL>30"
    "Convert TSV|plain|convert-tsv|query-tsv"
  )
fi

if [[ "$MODE" == "public-region-repeated" ]]; then
  CASES=(
    "Region QUAL|region|QUAL > 30|QUAL>30"
    "Region Convert TSV|region-convert-tsv|convert-tsv|query-tsv"
    "Region Stats JSON|region-stats-json|stats-json|stats"
  )
fi

if [[ "$MODE" == "public-heavy" ]]; then
  CASES=(
    "Heavy QUAL gzip input|gzip|QUAL > 30|QUAL>30"
    "Heavy Convert TSV gzip input|convert-tsv-gzip|convert-tsv|query-tsv"
  )
fi

if [[ "$MODE" == "stress" ]]; then
  CASES=(
    "QUAL plain|plain|QUAL > 30|QUAL>30"
    "DP plain|plain|DP > 40|INFO/DP>40"
    "AF plain|plain|AF > 0.2|INFO/AF>0.2"
    "FORMAT/DP > 20|plain|FORMAT/DP > 20|FMT/DP[0]>20|SAMPLE_001"
    "FORMAT/GQ >= 30|plain|FORMAT/GQ >= 30|FMT/GQ[0]>=30|SAMPLE_001"
    "FORMAT/GT == \"0/1\"|plain|FORMAT/GT == \"0/1\"|FMT/GT[0]=\"0/1\"|SAMPLE_001"
    "QUAL gzip input|gzip|QUAL > 30|QUAL>30"
    "Convert TSV|plain|convert-tsv|query-tsv"
    "Stats JSON|plain|stats-json|stats"
  )
fi

if [[ "$MODE" == "compatibility" ]]; then
  CASES=(
    "BCF input QUAL|bcf|QUAL > 30|QUAL>30"
    "BCF input TSV|bcf-convert-tsv|convert-tsv|query-tsv"
    "Indexed VCF region QUAL|region|QUAL > 30|QUAL>30"
    "Indexed BCF region stats|bcf-region-stats-json|stats-json|stats"
    "BGZF output QUAL|bgzf-output|QUAL > 30|QUAL>30"
  )
fi

{
  echo "## VCF-Fast Benchmark Report"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Mode: \`$MODE\`"
  echo "- Dataset source: $DATASET_SOURCE"
  echo "- Dataset source URL: see \`benchmark/download_public_data.sh\` for pinned GIAB/IGSR URLs when public modes are used"
  echo "- Dataset shape: $DATASET_SHAPE"
  echo "- Dataset sizes: \`${SIZES}\`"
  echo "- Public record tiers: \`${PUBLIC_RECORD_TIERS}\`"
  echo "- Heavy plain artifact cap: \`${HEAVY_MAX_PLAIN_BYTES}\` bytes"
  echo "- Heavy region: \`${HEAVY_REGION}\`"
  echo "- Repeated runs: \`${VCF_FAST_BENCH_RUNS:-3}\`"
  echo "- Warmup runs: \`${VCF_FAST_BENCH_WARMUP:-1}\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo "- Output equivalence: correctness result records whether VCF-Fast matched the bcftools baseline for the supported comparison."
  echo "- Throughput labels: vcf-fast variants/s and bcftools variants/s are reported together in the variants/sec column."
  echo "- Memory labels: vcf-fast peak RSS and bcftools peak RSS are reported together in the peak RSS column."
  echo
  echo "### Command Templates"
  echo
  echo "- VCF-Fast filter: \`./target/release/vcf-fast filter <input> --where '<expr>' -o <output>\`"
  echo "- VCF-Fast FORMAT filter: \`./target/release/vcf-fast filter <input> --sample SAMPLE_001 --where '<expr>' -o <output>\`"
  echo "- bcftools filter: \`bcftools filter -i '<expr>' <input> -o <output>\`"
  echo "- VCF-Fast convert TSV: \`./target/release/vcf-fast convert <input> --to tsv -o <output.tsv>\`"
  echo "- bcftools query TSV: \`bcftools query -u -f '%CHROM\\t%POS\\t%ID\\t%REF\\t%ALT\\t%QUAL\\t%FILTER\\t%INFO/DP\\t%INFO/AF\\n' <input>\`"
  echo "- VCF-Fast stats: \`./target/release/vcf-fast stats <input> > <output.json>\`"
  echo "- bcftools stats: \`bcftools stats <input> > <output.txt>\`"
  echo "- VCF-Fast region filter: \`./target/release/vcf-fast filter <input> --region '${PUBLIC_REGION}' --where '<expr>' -o <output>\`"
  echo "- bcftools region filter: \`bcftools view -r '${PUBLIC_REGION}' <input> | bcftools filter -i '<expr>' -Ov -o <output>\`"
  echo "- VCF-Fast BGZF output: \`./target/release/vcf-fast filter <input> --where '<expr>' --compression bgzf -o <output.vcf.gz>\`"
  echo "- BGZF validation: \`tabix -p vcf <output.vcf.gz> && bcftools view <output.vcf.gz> >/dev/null\`"
  echo "- Stress mode: \`VCF_FAST_BENCH_MODE=stress make bench-smoke\`"
  echo
  echo "| case | record count | dataset size bytes | input format | input compression | exact VCF-Fast command | exact competitor command | correctness result | vcf-fast mean | vcf-fast stddev | bcftools mean | bcftools stddev | speedup | variants/sec | peak RSS | caveats |"
  echo "|---|---:|---:|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|---|"
} >"$REPORT"

for records in $SIZES; do
  plain_dataset="$DATA_DIR/synthetic-${records}.vcf"
  if [[ "$MODE" == "synthetic" ]]; then
    ./benchmark/generate_synthetic_vcf.sh "$plain_dataset" "$records"
  elif [[ "$MODE" == "stress" ]]; then
    plain_dataset="$DATA_DIR/stress-${records}.vcf"
    ./benchmark/generate_stress_vcf.sh "$plain_dataset" "$records"
  elif [[ "$MODE" == "public-heavy" ]]; then
    gzip_dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
    set +e
    build_public_heavy_dataset "$PUBLIC_SOURCE" "$gzip_dataset" "$records" "$HEAVY_REGION"
    heavy_status=$?
    set -e
    if [[ "$heavy_status" -eq 77 ]]; then
      note="deferred: plain artifact cap exceeded"
      printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
        "public-heavy setup" "$records" "n/a" "VCF" "BGZF" "n/a" "n/a" "$note" "n/a" "n/a" "n/a" "n/a" "n/a" "n/a" "n/a" "$note" >>"$REPORT"
      continue
    fi
    if [[ "$heavy_status" -ne 0 ]]; then
      exit "$heavy_status"
    fi
    plain_dataset="$gzip_dataset"
  elif [[ "$MODE" == "public-region" || "$MODE" == "public-region-repeated" ]]; then
    plain_dataset="$DATA_DIR/${MODE}-${records}.vcf"
    build_public_region_dataset "$PUBLIC_SOURCE" "$plain_dataset" "$records" "$PUBLIC_REGION"
  elif [[ "$MODE" == "compatibility" ]]; then
    plain_dataset="$DATA_DIR/compatibility-${records}.vcf"
    unsorted_dataset="$DATA_DIR/compatibility-${records}.unsorted.vcf"
    ./benchmark/generate_synthetic_vcf.sh "$unsorted_dataset" "$records"
    sort_vcf_for_indexing "$unsorted_dataset" "$plain_dataset"
  else
    plain_dataset="$DATA_DIR/${MODE}-${records}.vcf"
    build_public_small_dataset "$PUBLIC_SOURCE" "$plain_dataset" "$records"
  fi
  if [[ "$MODE" != "public-heavy" ]]; then
    gzip_dataset="${plain_dataset}.gz"
  fi
  if [[ "$MODE" != "public-heavy" ]]; then
    gzip -c "$plain_dataset" >"$gzip_dataset"
  fi
  bgzf_dataset="${plain_dataset%.vcf}.bgzf.vcf.gz"
  bcf_dataset="${plain_dataset%.vcf}.bcf"

  if [[ "$MODE" == "compatibility" ]]; then
    require_tool bcftools
    require_tool bgzip
    require_tool tabix
    bgzip -c "$plain_dataset" >"$bgzf_dataset"
    tabix -f -p vcf "$bgzf_dataset"
    bcftools view -Ob -o "$bcf_dataset" "$plain_dataset"
    bcftools index -f "$bcf_dataset"
  fi
  if [[ "$MODE" == "public-heavy" ]]; then
    base_dataset_size_bytes="$(file_size_bytes "$gzip_dataset")"
  else
    base_dataset_size_bytes="$(file_size_bytes "$plain_dataset")"
  fi

  for case_spec in "${CASES[@]}"; do
    IFS='|' read -r case_name input_kind fast_expr bcftools_expr sample_name <<<"$case_spec"
    case_slug="$(slugify "$case_name")"
    dataset="$plain_dataset"
    dataset_size_bytes="$base_dataset_size_bytes"
    input_label="plain"
    input_format="VCF"
    input_compression="none"
    region_option=""
    output_compression_option=""
    fast_sample_option=""
    fast_sample_hyperfine_arg=""

    if [[ "$input_kind" == "gzip" || "$input_kind" == "convert-tsv-gzip" ]]; then
      dataset="$gzip_dataset"
      input_label="gzip"
      if [[ "$MODE" == "public-heavy" ]]; then
        input_compression="BGZF"
      else
        input_compression="gzip"
      fi
    elif [[ "$input_kind" == "bcf" || "$input_kind" == "bcf-convert-tsv" || "$input_kind" == "bcf-region-stats-json" ]]; then
      dataset="$bcf_dataset"
      input_label="bcf"
      input_format="BCF"
      input_compression="BGZF"
      dataset_size_bytes="$(file_size_bytes "$dataset")"
      if [[ "$input_kind" == "bcf-region-stats-json" ]]; then
        region_option="$COMPAT_REGION"
      fi
    elif [[ "$input_kind" == "region" || "$input_kind" == "region-convert-tsv" || "$input_kind" == "region-stats-json" ]]; then
      dataset="${PUBLIC_SOURCE:-$bgzf_dataset}"
      input_label="indexed-region"
      input_compression="BGZF"
      region_option="$PUBLIC_REGION"
      if [[ "$MODE" == "compatibility" ]]; then
        dataset="$bgzf_dataset"
        region_option="$COMPAT_REGION"
      fi
      dataset_size_bytes="$(file_size_bytes "$dataset")"
    elif [[ "$input_kind" == "bgzf-output" ]]; then
      input_label="plain-to-bgzf"
      output_compression_option="bgzf"
    fi

    if [[ -n "${sample_name:-}" ]]; then
      fast_sample_option="$sample_name"
      fast_sample_hyperfine_arg=" --sample $fast_sample_option"
    fi

    fast_out="$OUT_DIR/fast-${case_slug}-${records}.vcf"
    bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.vcf"
    fast_records="$OUT_DIR/fast-${case_slug}-${records}.records"
    bcftools_records="$OUT_DIR/bcftools-${case_slug}-${records}.records"
    hyperfine_json="$OUT_DIR/hyperfine-${case_slug}-${records}.json"

    note=""
    equivalence="vcf-fast predicate check"
    fast_mean="n/a"
    fast_stddev="n/a"
    bcftools_mean="n/a"
    bcftools_stddev="n/a"
    speedup="n/a"
    fast_variants_per_second="n/a"
    bcftools_variants_per_second="n/a"
    fast_peak_rss_kb="n/a"
    bcftools_peak_rss_kb="n/a"
    fast_command=""
    competitor_command=""

    if [[ "$fast_expr" == "stats-json" ]]; then
      fast_out="$OUT_DIR/fast-${case_slug}-${records}.json"
      bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.stats.txt"
      if [[ -n "$region_option" ]]; then
        fast_command="./target/release/vcf-fast stats $dataset --region $region_option > $fast_out"
        ./target/release/vcf-fast stats "$dataset" --region "$region_option" >"$fast_out"
      else
        fast_command="./target/release/vcf-fast stats $dataset > $fast_out"
        ./target/release/vcf-fast stats "$dataset" >"$fast_out"
      fi
      if command -v bcftools >/dev/null 2>&1; then
        if [[ -n "$region_option" ]]; then
          competitor_command="bcftools view -r $region_option $dataset | bcftools stats - > $bcftools_out"
          bcftools view -r "$region_option" "$dataset" | bcftools stats - >"$bcftools_out"
        else
          competitor_command="bcftools stats $dataset > $bcftools_out"
          bcftools stats "$dataset" >"$bcftools_out"
        fi
        compare_stats_counts "$fast_out" "$bcftools_out" "$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="Stats JSON variants match bcftools stats records"
      else
        note="bcftools unavailable"
      fi
    elif [[ "$fast_expr" == "convert-tsv" ]]; then
      fast_out="$OUT_DIR/fast-${case_slug}-${records}.tsv"
      bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.tsv"
      if [[ -n "$region_option" ]]; then
        fast_command="./target/release/vcf-fast convert $dataset --region $region_option --to tsv -o $fast_out"
        ./target/release/vcf-fast convert "$dataset" --region "$region_option" --to tsv -o "$fast_out"
      else
        fast_command="./target/release/vcf-fast convert $dataset --to tsv -o $fast_out"
        ./target/release/vcf-fast convert "$dataset" --to tsv -o "$fast_out"
      fi
      if command -v bcftools >/dev/null 2>&1; then
        if [[ -n "$region_option" ]]; then
          competitor_command="bcftools view -r $region_option $dataset | bcftools query -u -f '%CHROM\\t%POS\\t%ID\\t%REF\\t%ALT\\t%QUAL\\t%FILTER\\t%INFO/DP\\t%INFO/AF\\n' > $bcftools_out"
          bcftools view -r "$region_option" "$dataset" | bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' >"$bcftools_out"
        else
          competitor_command="bcftools query -u -f '%CHROM\\t%POS\\t%ID\\t%REF\\t%ALT\\t%QUAL\\t%FILTER\\t%INFO/DP\\t%INFO/AF\\n' $dataset > $bcftools_out"
          bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' "$dataset" >"$bcftools_out"
        fi
        python3 benchmark/normalize_tsv.py --skip-header "$fast_out" >"$fast_records"
        python3 benchmark/normalize_tsv.py "$bcftools_out" >"$bcftools_records"
        diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="matches normalized bcftools query TSV rows"
      else
        note="bcftools unavailable"
      fi
    else
      if [[ -n "$fast_sample_option" ]]; then
        fast_command="./target/release/vcf-fast filter $dataset --sample $fast_sample_option --where '$fast_expr' -o $fast_out"
        ./target/release/vcf-fast filter "$dataset" --sample "$fast_sample_option" --where "$fast_expr" -o "$fast_out"
      elif [[ -n "$region_option" ]]; then
        fast_command="./target/release/vcf-fast filter $dataset --region $region_option --where '$fast_expr' -o $fast_out"
        ./target/release/vcf-fast filter "$dataset" --region "$region_option" --where "$fast_expr" -o "$fast_out"
      elif [[ "$output_compression_option" == "bgzf" ]]; then
        fast_out="$OUT_DIR/fast-${case_slug}-${records}.vcf.gz"
        fast_command="./target/release/vcf-fast filter $dataset --where '$fast_expr' --compression bgzf -o $fast_out"
        ./target/release/vcf-fast filter "$dataset" --where "$fast_expr" --compression bgzf -o "$fast_out"
      else
        fast_command="./target/release/vcf-fast filter $dataset --where '$fast_expr' -o $fast_out"
        ./target/release/vcf-fast filter "$dataset" --where "$fast_expr" -o "$fast_out"
      fi
      if [[ "$output_compression_option" == "bgzf" ]]; then
        gzip -t "$fast_out"
        tabix -f -p vcf "$fast_out"
        bcftools view "$fast_out" >/dev/null
        bcftools view "$fast_out" -Ov -o "$OUT_DIR/fast-${case_slug}-${records}.decompressed.vcf"
        predicate_check "$OUT_DIR/fast-${case_slug}-${records}.decompressed.vcf" "$fast_expr"
      else
        predicate_check "$fast_out" "$fast_expr"
      fi

      if command -v bcftools >/dev/null 2>&1; then
        if [[ -n "$region_option" ]]; then
          competitor_command="bcftools view -r $region_option $dataset | bcftools filter -i '$bcftools_expr' -Ov -o $bcftools_out"
          bcftools view -r "$region_option" "$dataset" | bcftools filter -i "$bcftools_expr" -Ov -o "$bcftools_out"
        elif [[ "$output_compression_option" == "bgzf" ]]; then
          competitor_command="bcftools filter -i '$bcftools_expr' $dataset -Oz -o $bcftools_out.gz"
          bcftools filter -i "$bcftools_expr" "$dataset" -Oz -o "$bcftools_out.gz"
          bcftools view "$bcftools_out.gz" -Ov -o "$bcftools_out"
        else
          competitor_command="bcftools filter -i '$bcftools_expr' $dataset -o $bcftools_out"
          bcftools filter -i "$bcftools_expr" "$dataset" -o "$bcftools_out"
        fi
        if [[ "$output_compression_option" == "bgzf" ]]; then
          extract_core_records "$OUT_DIR/fast-${case_slug}-${records}.decompressed.vcf" >"$fast_records"
        else
          extract_core_records "$fast_out" >"$fast_records"
        fi
        extract_core_records "$bcftools_out" >"$bcftools_records"
        diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="matches bcftools filtered core records"
      else
        note="bcftools unavailable"
      fi
    fi

    if command -v hyperfine >/dev/null 2>&1; then
      if command -v bcftools >/dev/null 2>&1 && [[ "$fast_expr" == "stats-json" ]]; then
        if [[ -n "$region_option" ]]; then
          hyperfine \
            --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
            --runs "${VCF_FAST_BENCH_RUNS:-3}" \
            --export-json "$hyperfine_json" \
            "./target/release/vcf-fast stats $dataset --region $region_option > $OUT_DIR/fast-${case_slug}-${records}.timed.json" \
            "bcftools view -r $region_option $dataset | bcftools stats - > $OUT_DIR/bcftools-${case_slug}-${records}.timed.stats.txt"
        else
          hyperfine \
            --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
            --runs "${VCF_FAST_BENCH_RUNS:-3}" \
            --export-json "$hyperfine_json" \
            "./target/release/vcf-fast stats $dataset > $OUT_DIR/fast-${case_slug}-${records}.timed.json" \
            "bcftools stats $dataset > $OUT_DIR/bcftools-${case_slug}-${records}.timed.stats.txt"
        fi
        read -r fast_mean fast_stddev bcftools_mean bcftools_stddev speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$3" ]]; then ./target/release/vcf-fast stats "$1" --region "$3" > "$2"; else ./target/release/vcf-fast stats "$1" > "$2"; fi' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.json" "$region_option")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'if [[ -n "$3" ]]; then bcftools view -r "$3" "$1" | bcftools stats - > "$2"; else bcftools stats "$1" > "$2"; fi' _ "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.stats.txt" "$region_option")"
      elif command -v bcftools >/dev/null 2>&1 && [[ "$fast_expr" == "convert-tsv" ]]; then
        if [[ -n "$region_option" ]]; then
          hyperfine \
            --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
            --runs "${VCF_FAST_BENCH_RUNS:-3}" \
            --export-json "$hyperfine_json" \
            "./target/release/vcf-fast convert $dataset --region $region_option --to tsv -o $OUT_DIR/fast-${case_slug}-${records}.timed.tsv" \
            "bcftools view -r $region_option $dataset | bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' > $OUT_DIR/bcftools-${case_slug}-${records}.timed.tsv"
        else
          hyperfine \
            --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
            --runs "${VCF_FAST_BENCH_RUNS:-3}" \
            --export-json "$hyperfine_json" \
            "./target/release/vcf-fast convert $dataset --to tsv -o $OUT_DIR/fast-${case_slug}-${records}.timed.tsv" \
            "bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' $dataset > $OUT_DIR/bcftools-${case_slug}-${records}.timed.tsv"
        fi
        read -r fast_mean fast_stddev bcftools_mean bcftools_stddev speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$3" ]]; then ./target/release/vcf-fast convert "$1" --region "$3" --to tsv -o "$2"; else ./target/release/vcf-fast convert "$1" --to tsv -o "$2"; fi' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.tsv" "$region_option")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'if [[ -n "$3" ]]; then bcftools view -r "$3" "$1" | bcftools query -u -f "%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n" > "$2"; else bcftools query -u -f "%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n" "$1" > "$2"; fi' _ "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.tsv" "$region_option")"
      elif command -v bcftools >/dev/null 2>&1; then
        fast_timed_output="$OUT_DIR/fast-${case_slug}-${records}.timed.vcf"
        fast_timed_command="./target/release/vcf-fast filter $dataset$fast_sample_hyperfine_arg --where '$fast_expr' -o $fast_timed_output"
        bcftools_timed_command="bcftools filter -i '$bcftools_expr' $dataset -o $OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf"
        if [[ -n "$region_option" ]]; then
          fast_timed_command="./target/release/vcf-fast filter $dataset --region $region_option --where '$fast_expr' -o $fast_timed_output"
          bcftools_timed_command="bcftools view -r $region_option $dataset | bcftools filter -i '$bcftools_expr' -Ov -o $OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf"
        elif [[ "$output_compression_option" == "bgzf" ]]; then
          fast_timed_output="$OUT_DIR/fast-${case_slug}-${records}.timed.vcf.gz"
          fast_timed_command="./target/release/vcf-fast filter $dataset --where '$fast_expr' --compression bgzf -o $fast_timed_output"
          bcftools_timed_command="bcftools filter -i '$bcftools_expr' $dataset -Oz -o $OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf.gz"
        fi
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          --export-json "$hyperfine_json" \
          "$fast_timed_command" \
          "$bcftools_timed_command"
        read -r fast_mean fast_stddev bcftools_mean bcftools_stddev speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$6" ]]; then ./target/release/vcf-fast filter "$1" --where "$2" --compression bgzf -o "$3"; elif [[ -n "$5" ]]; then ./target/release/vcf-fast filter "$1" --region "$5" --where "$2" -o "$3"; elif [[ -n "$4" ]]; then ./target/release/vcf-fast filter "$1" --sample "$4" --where "$2" -o "$3"; else ./target/release/vcf-fast filter "$1" --where "$2" -o "$3"; fi' _ "$dataset" "$fast_expr" "$OUT_DIR/fast-${case_slug}-${records}.rss.vcf" "${sample_name:-}" "$region_option" "$output_compression_option")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'if [[ -n "$4" ]]; then bcftools view -r "$4" "$2" | bcftools filter -i "$1" -Ov -o "$3"; elif [[ -n "$5" ]]; then bcftools filter -i "$1" "$2" -Oz -o "$3.gz"; else bcftools filter -i "$1" "$2" -o "$3"; fi' _ "$bcftools_expr" "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.vcf" "$region_option" "$output_compression_option")"
      elif [[ "$fast_expr" == "stats-json" ]]; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          "./target/release/vcf-fast stats $dataset > $OUT_DIR/fast-${case_slug}-${records}.timed.json"
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c './target/release/vcf-fast stats "$1" > "$2"' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.json")"
      elif [[ "$fast_expr" == "convert-tsv" ]]; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          "./target/release/vcf-fast convert $dataset --to tsv -o $OUT_DIR/fast-${case_slug}-${records}.timed.tsv"
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c './target/release/vcf-fast convert "$1" --to tsv -o "$2"' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.tsv")"
      else
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          "./target/release/vcf-fast filter $dataset$fast_sample_hyperfine_arg --where '$fast_expr' -o $OUT_DIR/fast-${case_slug}-${records}.timed.vcf"
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$4" ]]; then ./target/release/vcf-fast filter "$1" --sample "$4" --where "$2" -o "$3"; else ./target/release/vcf-fast filter "$1" --where "$2" -o "$3"; fi' _ "$dataset" "$fast_expr" "$OUT_DIR/fast-${case_slug}-${records}.rss.vcf" "${sample_name:-}")"
      fi
    else
      note="${note:+$note; }hyperfine unavailable"
    fi

    fast_variants_per_second="$(variants_per_second "$records" "$fast_mean")"
    bcftools_variants_per_second="$(variants_per_second "$records" "$bcftools_mean")"
    [[ -n "$fast_command" ]] || fast_command="n/a"
    [[ -n "$competitor_command" ]] || competitor_command="bcftools unavailable"
    fast_command_cell="$(markdown_cell "\`$fast_command\`")"
    competitor_command_cell="$(markdown_cell "\`$competitor_command\`")"
    variants_per_second_cell="${fast_variants_per_second} / ${bcftools_variants_per_second}"
    peak_rss_cell="${fast_peak_rss_kb} / ${bcftools_peak_rss_kb}"

    printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
      "$case_name" "$records" "$dataset_size_bytes" "$input_format" "$input_compression" "$fast_command_cell" "$competitor_command_cell" "$equivalence" "$fast_mean" "$fast_stddev" "$bcftools_mean" "$bcftools_stddev" "$speedup" "$variants_per_second_cell" "$peak_rss_cell" "${note:-}" >>"$REPORT"
  done
done

{
  echo
  echo "### Raw Artifacts"
  echo
  echo "- Working datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-*.json\`"
  echo "- Peak RSS files: \`$OUT_DIR/rss-*.txt\`"
  echo "- Equivalence diffs: \`$OUT_DIR/equivalence-*.diff\`"
} >>"$REPORT"

echo "Benchmark report written to $REPORT"
