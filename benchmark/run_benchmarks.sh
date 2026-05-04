#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results}"
DATA_DIR="$OUT_DIR/data"
REPORT="$OUT_DIR/benchmark-report.md"
SIZES="${VCF_FAST_BENCH_SIZES:-10000 100000}"
CASES=(
  "QUAL plain|plain|QUAL > 30|QUAL>30"
  "DP plain|plain|DP > 40|INFO/DP>40"
  "AF plain|plain|AF > 0.2|INFO/AF>0.2"
  "QUAL gzip input|gzip|QUAL > 30|QUAL>30"
)

tool_version() {
  local tool="$1"
  if command -v "$tool" >/dev/null 2>&1; then
    "$tool" --version 2>&1 | head -n 1
  else
    echo "unavailable"
  fi
}

extract_core_records() {
  awk -F '\t' 'BEGIN { OFS = "\t" } !/^#/ { print $1, $2, $3, $4, $5, $6, $7 }' "$1"
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
    *)
      echo "no predicate check defined for $expression" >&2
      return 1
      ;;
  esac
}

mkdir -p "$OUT_DIR" "$DATA_DIR"
cargo build --release

{
  echo "## VCF-Fast Benchmark Report"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Dataset sizes: \`$SIZES\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |"
  echo "|---|---:|---|---|---:|---:|---:|---|"
} >"$REPORT"

for records in $SIZES; do
  plain_dataset="$DATA_DIR/synthetic-${records}.vcf"
  gzip_dataset="$DATA_DIR/synthetic-${records}.vcf.gz"
  ./benchmark/generate_synthetic_vcf.sh "$plain_dataset" "$records"
  gzip -c "$plain_dataset" >"$gzip_dataset"

  for case_spec in "${CASES[@]}"; do
    IFS='|' read -r case_name input_kind fast_expr bcftools_expr <<<"$case_spec"
    case_slug="$(slugify "$case_name")"
    dataset="$plain_dataset"
    input_label="plain"

    if [[ "$input_kind" == "gzip" ]]; then
      dataset="$gzip_dataset"
      input_label="gzip"
    fi

    fast_out="$OUT_DIR/fast-${case_slug}-${records}.vcf"
    bcftools_out="$OUT_DIR/bcftools-${case_slug}-${records}.vcf"
    fast_records="$OUT_DIR/fast-${case_slug}-${records}.records"
    bcftools_records="$OUT_DIR/bcftools-${case_slug}-${records}.records"
    hyperfine_json="$OUT_DIR/hyperfine-${case_slug}-${records}.json"

    ./target/release/vcf-fast filter "$dataset" --where "$fast_expr" -o "$fast_out"
    predicate_check "$fast_out" "$fast_expr"

    note=""
    equivalence="vcf-fast predicate check"
    fast_mean="n/a"
    bcftools_mean="n/a"
    speedup="n/a"

    if command -v bcftools >/dev/null 2>&1; then
      bcftools filter -i "$bcftools_expr" "$dataset" -o "$bcftools_out"
      extract_core_records "$fast_out" >"$fast_records"
      extract_core_records "$bcftools_out" >"$bcftools_records"
      diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${case_slug}-${records}.diff"
      equivalence="matches bcftools filtered core records"
    else
      note="bcftools unavailable"
    fi

    if command -v hyperfine >/dev/null 2>&1; then
      if command -v bcftools >/dev/null 2>&1; then
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          --export-json "$hyperfine_json" \
          "./target/release/vcf-fast filter $dataset --where '$fast_expr' -o $OUT_DIR/fast-${case_slug}-${records}.timed.vcf" \
          "bcftools filter -i '$bcftools_expr' $dataset -o $OUT_DIR/bcftools-${case_slug}-${records}.timed.vcf"
        read -r fast_mean bcftools_mean speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
      else
        hyperfine \
          --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
          --runs "${VCF_FAST_BENCH_RUNS:-3}" \
          "./target/release/vcf-fast filter $dataset --where '$fast_expr' -o $OUT_DIR/fast-${case_slug}-${records}.timed.vcf"
      fi
    else
      note="${note:+$note; }hyperfine unavailable"
    fi

    printf '| %s | %s | %s | %s | %s | %s | %s | %s |\n' \
      "$case_name" "$records" "$input_label" "$equivalence" "$fast_mean" "$bcftools_mean" "$speedup" "${note:-}" >>"$REPORT"
  done
done

{
  echo
  echo "### Raw Artifacts"
  echo
  echo "- Synthetic datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-*.json\`"
  echo "- Equivalence diffs: \`$OUT_DIR/equivalence-*.diff\`"
} >>"$REPORT"

echo "Benchmark report written to $REPORT"
