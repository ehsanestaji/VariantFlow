#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results}"
DATA_DIR="$OUT_DIR/data"
REPORT="$OUT_DIR/benchmark-report.md"
MODE="${VCF_FAST_BENCH_MODE:-synthetic}"
SIZES="${VCF_FAST_BENCH_SIZES:-10000 100000}"
PUBLIC_RECORDS="${VCF_FAST_PUBLIC_RECORDS:-10000}"
PUBLIC_REGION="${VCF_FAST_PUBLIC_REGION:-chr22:1-20000000}"
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
    stress)
      echo "stress synthetic data|"
      ;;
    *)
      echo "unsupported VCF_FAST_BENCH_MODE=$MODE; expected synthetic, stress, public-small, or public-region" >&2
      exit 2
      ;;
  esac
}

mkdir -p "$OUT_DIR" "$DATA_DIR"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi
cargo build --release
IFS='|' read -r DATASET_SOURCE PUBLIC_SOURCE < <(configure_inputs)
DATASET_SHAPE="records only"

if [[ "$MODE" == "stress" ]]; then
  DATASET_SHAPE="stress INFO fields=${STRESS_INFO_FIELDS}, samples=${STRESS_SAMPLES}, FORMAT=GT:DP:GQ:AD"
fi

if [[ "$MODE" != "synthetic" ]]; then
  CASES=(
    "QUAL plain|plain|QUAL > 30|QUAL>30"
    "QUAL gzip input|gzip|QUAL > 30|QUAL>30"
    "Convert TSV|plain|convert-tsv|query-tsv"
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

{
  echo "## VCF-Fast Benchmark Report"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Mode: \`$MODE\`"
  echo "- Dataset source: $DATASET_SOURCE"
  echo "- Dataset shape: $DATASET_SHAPE"
  echo "- Dataset sizes: \`$SIZES\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
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
  echo "- Stress mode: \`VCF_FAST_BENCH_MODE=stress make bench-smoke\`"
  echo
  echo "| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | vcf-fast variants/s | bcftools variants/s | vcf-fast peak RSS KB | bcftools peak RSS KB | notes |"
  echo "|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---|"
} >"$REPORT"

for records in $SIZES; do
  plain_dataset="$DATA_DIR/synthetic-${records}.vcf"
  if [[ "$MODE" == "synthetic" ]]; then
    ./benchmark/generate_synthetic_vcf.sh "$plain_dataset" "$records"
  elif [[ "$MODE" == "stress" ]]; then
    plain_dataset="$DATA_DIR/stress-${records}.vcf"
    ./benchmark/generate_stress_vcf.sh "$plain_dataset" "$records"
  elif [[ "$MODE" == "public-region" ]]; then
    plain_dataset="$DATA_DIR/${MODE}-${records}.vcf"
    build_public_region_dataset "$PUBLIC_SOURCE" "$plain_dataset" "$records" "$PUBLIC_REGION"
  else
    plain_dataset="$DATA_DIR/${MODE}-${records}.vcf"
    build_public_small_dataset "$PUBLIC_SOURCE" "$plain_dataset" "$records"
  fi
  gzip_dataset="${plain_dataset}.gz"
  gzip -c "$plain_dataset" >"$gzip_dataset"

  for case_spec in "${CASES[@]}"; do
    IFS='|' read -r case_name input_kind fast_expr bcftools_expr sample_name <<<"$case_spec"
    case_slug="$(slugify "$case_name")"
    dataset="$plain_dataset"
    input_label="plain"
    fast_sample_option=""
    fast_sample_hyperfine_arg=""

    if [[ "$input_kind" == "gzip" ]]; then
      dataset="$gzip_dataset"
      input_label="gzip"
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
    bcftools_mean="n/a"
    speedup="n/a"
    fast_variants_per_second="n/a"
    bcftools_variants_per_second="n/a"
    fast_peak_rss_kb="n/a"
    bcftools_peak_rss_kb="n/a"

    if [[ "$fast_expr" == "stats-json" ]]; then
      fast_out="$OUT_DIR/fast-${case_slug}-${records}.json"
      bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.stats.txt"
      ./target/release/vcf-fast stats "$dataset" >"$fast_out"
      if command -v bcftools >/dev/null 2>&1; then
        bcftools stats "$dataset" >"$bcftools_out"
        compare_stats_counts "$fast_out" "$bcftools_out" "$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="Stats JSON variants match bcftools stats records"
      else
        note="bcftools unavailable"
      fi
    elif [[ "$fast_expr" == "convert-tsv" ]]; then
      fast_out="$OUT_DIR/fast-${case_slug}-${records}.tsv"
      bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.tsv"
      ./target/release/vcf-fast convert "$dataset" --to tsv -o "$fast_out"
      if command -v bcftools >/dev/null 2>&1; then
        bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' "$dataset" >"$bcftools_out"
        python3 benchmark/normalize_tsv.py --skip-header "$fast_out" >"$fast_records"
        python3 benchmark/normalize_tsv.py "$bcftools_out" >"$bcftools_records"
        diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="matches normalized bcftools query TSV rows"
      else
        note="bcftools unavailable"
      fi
    else
      if [[ -n "$fast_sample_option" ]]; then
        ./target/release/vcf-fast filter "$dataset" --sample "$fast_sample_option" --where "$fast_expr" -o "$fast_out"
      else
        ./target/release/vcf-fast filter "$dataset" --where "$fast_expr" -o "$fast_out"
      fi
      predicate_check "$fast_out" "$fast_expr"

      if command -v bcftools >/dev/null 2>&1; then
        bcftools filter -i "$bcftools_expr" "$dataset" -o "$bcftools_out"
        extract_core_records "$fast_out" >"$fast_records"
        extract_core_records "$bcftools_out" >"$bcftools_records"
        diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${case_slug}-${records}.diff"
        equivalence="matches bcftools filtered core records"
      else
        note="bcftools unavailable"
      fi
    fi

    if command -v hyperfine >/dev/null 2>&1; then
      if command -v bcftools >/dev/null 2>&1 && [[ "$fast_expr" == "stats-json" ]]; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          --export-json "$hyperfine_json" \
          "./target/release/vcf-fast stats $dataset > $OUT_DIR/fast-${case_slug}-${records}.timed.json" \
          "bcftools stats $dataset > $OUT_DIR/bcftools-${case_slug}-${records}.timed.stats.txt"
        read -r fast_mean bcftools_mean speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c './target/release/vcf-fast stats "$1" > "$2"' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.json")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'bcftools stats "$1" > "$2"' _ "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.stats.txt")"
      elif command -v bcftools >/dev/null 2>&1 && [[ "$fast_expr" == "convert-tsv" ]]; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          --export-json "$hyperfine_json" \
          "./target/release/vcf-fast convert $dataset --to tsv -o $OUT_DIR/fast-${case_slug}-${records}.timed.tsv" \
          "bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' $dataset > $OUT_DIR/bcftools-${case_slug}-${records}.timed.tsv"
        read -r fast_mean bcftools_mean speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c './target/release/vcf-fast convert "$1" --to tsv -o "$2"' _ "$dataset" "$OUT_DIR/fast-${case_slug}-${records}.rss.tsv")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'bcftools query -u -f "%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n" "$1" > "$2"' _ "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.tsv")"
      elif command -v bcftools >/dev/null 2>&1; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          --export-json "$hyperfine_json" \
          "./target/release/vcf-fast filter $dataset$fast_sample_hyperfine_arg --where '$fast_expr' -o $OUT_DIR/fast-${case_slug}-${records}.timed.vcf" \
          "bcftools filter -i '$bcftools_expr' $dataset -o $OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf"
        read -r fast_mean bcftools_mean speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
        fast_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-fast-${case_slug}-${records}.txt" bash -c 'if [[ -n "$4" ]]; then ./target/release/vcf-fast filter "$1" --sample "$4" --where "$2" -o "$3"; else ./target/release/vcf-fast filter "$1" --where "$2" -o "$3"; fi' _ "$dataset" "$fast_expr" "$OUT_DIR/fast-${case_slug}-${records}.rss.vcf" "${sample_name:-}")"
        bcftools_peak_rss_kb="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${case_slug}-${records}.txt" bash -c 'bcftools filter -i "$1" "$2" -o "$3"' _ "$bcftools_expr" "$dataset" "$OUT_DIR/bcftools-${case_slug}-${records}.rss.vcf")"
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

    printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
      "$case_name" "$records" "$input_label" "$equivalence" "$fast_mean" "$bcftools_mean" "$speedup" "$fast_variants_per_second" "$bcftools_variants_per_second" "$fast_peak_rss_kb" "$bcftools_peak_rss_kb" "${note:-}" >>"$REPORT"
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
