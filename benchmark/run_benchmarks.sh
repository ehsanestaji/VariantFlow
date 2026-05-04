#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results}"
DATA_DIR="$OUT_DIR/data"
REPORT="$OUT_DIR/benchmark-report.md"
SIZES="${VCF_FAST_BENCH_SIZES:-10000 100000}"
FILTER_EXPR='QUAL > 30'
BCFTOOLS_EXPR='QUAL>30'

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

mkdir -p "$OUT_DIR" "$DATA_DIR"
cargo build --release

{
  echo "## VCF-Fast Benchmark Report"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Filter expression: \`$FILTER_EXPR\`"
  echo "- Dataset sizes: \`$SIZES\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo
  echo "| records | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |"
  echo "|---:|---|---:|---:|---:|---|"
} >"$REPORT"

for records in $SIZES; do
  dataset="$DATA_DIR/synthetic-${records}.vcf"
  fast_out="$OUT_DIR/fast-${records}.vcf"
  bcftools_out="$OUT_DIR/bcftools-${records}.vcf"
  fast_records="$OUT_DIR/fast-${records}.records"
  bcftools_records="$OUT_DIR/bcftools-${records}.records"
  hyperfine_json="$OUT_DIR/hyperfine-${records}.json"

  ./benchmark/generate_synthetic_vcf.sh "$dataset" "$records"

  ./target/release/vcf-fast filter "$dataset" --where "$FILTER_EXPR" -o "$fast_out"
  awk -F '\t' 'BEGIN { ok = 1 } /^#/ { next } $6 <= 30 { ok = 0; print "record failed QUAL > 30: " $0 > "/dev/stderr" } END { exit ok ? 0 : 1 }' "$fast_out"

  note=""
  equivalence="vcf-fast predicate check"
  fast_mean="n/a"
  bcftools_mean="n/a"
  speedup="n/a"

  if command -v bcftools >/dev/null 2>&1; then
    bcftools filter -i "$BCFTOOLS_EXPR" "$dataset" -o "$bcftools_out"
    extract_core_records "$fast_out" >"$fast_records"
    extract_core_records "$bcftools_out" >"$bcftools_records"
    diff -u "$bcftools_records" "$fast_records" >"$OUT_DIR/equivalence-${records}.diff"
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
        "./target/release/vcf-fast filter $dataset --where '$FILTER_EXPR' -o $OUT_DIR/fast-${records}.timed.vcf" \
        "bcftools filter -i '$BCFTOOLS_EXPR' $dataset -o $OUT_DIR/bcftools-${records}.timed.vcf"
      read -r fast_mean bcftools_mean speedup < <(python3 benchmark/summarize_hyperfine.py "$hyperfine_json")
    else
      hyperfine \
        --warmup "${VCF_FAST_BENCH_WARMUP:-1}" \
        --runs "${VCF_FAST_BENCH_RUNS:-3}" \
        "./target/release/vcf-fast filter $dataset --where '$FILTER_EXPR' -o $OUT_DIR/fast-${records}.timed.vcf"
    fi
  else
    note="${note:+$note; }hyperfine unavailable"
  fi

  printf '| %s | %s | %s | %s | %s | %s |\n' \
    "$records" "$equivalence" "$fast_mean" "$bcftools_mean" "$speedup" "${note:-}" >>"$REPORT"
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
