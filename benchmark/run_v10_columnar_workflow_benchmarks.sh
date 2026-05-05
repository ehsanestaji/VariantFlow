#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v10-columnar-workflow}"
DATA_DIR="$OUT_DIR/data"
REPORT="${VCF_FAST_V10_COLUMNAR_REPORT:-benchmark/reports/v10-columnar-workflow-benchmark.md}"
MODE="${VCF_FAST_COLUMNAR_MODE:-stress}"
SIZES="${VCF_FAST_V10_COLUMNAR_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
REPEATED_QUERIES="${VCF_FAST_COLUMNAR_REPEATED_QUERIES:-5}"
PUBLIC_REGION="${VCF_FAST_PUBLIC_REGION:-chr22:1-20000000}"
IGSR_SOURCE="${VCF_FAST_IGSR_SOURCE:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
PYTHON="${VCF_FAST_PYTHON:-python3}"
QUERY="${VCF_FAST_COLUMNAR_QUERY:-auto}"

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "$tool is required for v1.0 columnar workflow evidence" >&2
    exit 2
  fi
}

tool_version() {
  local tool="$1"
  "$tool" --version 2>&1 | head -n 1
}

file_size_bytes() {
  wc -c <"$1" | tr -d ' '
}

slugify() {
  tr '[:upper:] ' '[:lower:]-' <<<"$1" | tr -cd 'a-z0-9-'
}

escape_markdown_table_cell() {
  sed 's/|/\\|/g' <<<"$1"
}

variants_per_second() {
  local records="$1"
  local mean="$2"
  "$PYTHON" - "$records" "$mean" <<'PY'
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
    "$@" >/dev/null
    echo "n/a"
  fi
}

summarize_hyperfine() {
  local json="$1"
  "$PYTHON" - "$json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    results = json.load(handle)["results"]

def fmt(value):
    return f"{float(value or 0):.6f}s"

export, duckdb, bcftools = results
export_mean = float(export["mean"])
duckdb_mean = float(duckdb["mean"])
bcftools_mean = float(bcftools["mean"])
workflow_mean = export_mean + duckdb_mean
print(
    fmt(export_mean),
    fmt(export.get("stddev")),
    fmt(duckdb_mean),
    fmt(duckdb.get("stddev")),
    fmt(bcftools_mean),
    fmt(bcftools.get("stddev")),
    f"{bcftools_mean / duckdb_mean:.2f}x" if duckdb_mean > 0 else "n/a",
    f"{bcftools_mean / workflow_mean:.2f}x" if workflow_mean > 0 else "n/a",
)
PY
}

stage_public_heavy_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"
  local region="$4"

  if [[ ! -s "$source" ]]; then
    echo "missing $source; run benchmark/download_public_data.sh igsr-chr22 first" >&2
    exit 2
  fi
  require_tool bgzip
  require_tool tabix
  {
    bcftools view -h "$source"
    bcftools view -H -r "$region" "$source" | awk -v max="$records" 'NR <= max'
  } | bgzip -c >"$output"
  tabix -f -p vcf "$output"
}

repeated_bcftools_count() {
  local input="$1"
  local repeats="$2"
  local expr="$3"
  local result="0"
  for _ in $(seq 1 "$repeats"); do
    if [[ "$expr" == "__ROW_COUNT__" ]]; then
      result="$(bcftools view -H "$input" | wc -l | tr -d ' ')"
    else
      result="$(bcftools filter -i "$expr" "$input" -Ou | bcftools view -H | wc -l | tr -d ' ')"
    fi
  done
  echo "$result"
}

require_tool bcftools
require_tool hyperfine
require_tool "$PYTHON"
"$PYTHON" benchmark/query_parquet_duckdb.py --check

if [[ "$MODE" != "stress" && "$MODE" != "public-heavy" ]]; then
  echo "unsupported VCF_FAST_COLUMNAR_MODE=$MODE; expected stress or public-heavy" >&2
  exit 2
fi

mkdir -p "$OUT_DIR" "$DATA_DIR" "$(dirname "$REPORT")"
if /usr/bin/time -v -o "$OUT_DIR/time-probe.txt" true >/dev/null 2>&1; then
  VCF_FAST_BENCH_GNU_TIME=1
else
  VCF_FAST_BENCH_GNU_TIME=0
fi

cargo build --release

{
  echo "# VCF-Fast v1.0 Columnar Workflow Benchmark"
  echo
  echo "## Status"
  echo
  echo "This report tests the Parquet workflow claim: export once, then run repeated queries through DuckDB. It compares repeated DuckDB queries over VCF-Fast Parquet output against repeated \`bcftools\` scans over the original VCF/BGZF input. It does not replace the native selective filter claim."
  echo
  echo "## Run Configuration"
  echo
  echo "- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "- Mode: \`$MODE\`"
  echo "- Dataset source: deterministic stress data or bounded IGSR public-heavy data"
  echo "- Dataset source URL: see \`benchmark/download_public_data.sh\` for the pinned IGSR source when \`public-heavy\` is used"
  echo "- Public-heavy region: \`$PUBLIC_REGION\`"
  echo "- Record tiers: \`$SIZES\`"
  echo "- Repeated queries: \`$REPEATED_QUERIES\`"
  echo "- Query selector: \`$QUERY\`"
  echo "- Repeated runs: \`$RUNS\`"
  echo "- Warmup runs: \`$WARMUP\`"
  echo "- hyperfine: $(tool_version hyperfine)"
  echo "- bcftools: $(tool_version bcftools)"
  echo "- DuckDB: $("$PYTHON" - <<'PY'
import duckdb
print(duckdb.__version__)
PY
)"
  echo
  echo "## Measured Workflow Cases"
  echo
  echo "| case | dataset source | dataset size bytes | record count | exact export command | exact DuckDB command | exact competitor command | correctness result | export mean/stddev | DuckDB repeated query mean/stddev | bcftools repeated scan mean/stddev | query-only speedup | amortized speedup | variants/sec | peak RSS | caveat | claim decision |"
  echo "| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |"
} >"$REPORT"

for records in $SIZES; do
  if [[ "$MODE" == "stress" ]]; then
    dataset="$DATA_DIR/stress-${records}.vcf"
    dataset_source="deterministic stress VCF"
    ./benchmark/generate_stress_vcf.sh "$dataset" "$records"
    selected_query="${QUERY}"
    if [[ "$selected_query" == "auto" ]]; then
      selected_query="qual_gt_30"
    fi
  else
    dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
    dataset_source="bounded IGSR chr22 public-heavy BGZF"
    stage_public_heavy_dataset "$IGSR_SOURCE" "$dataset" "$records" "$PUBLIC_REGION"
    selected_query="${QUERY}"
    if [[ "$selected_query" == "auto" ]]; then
      selected_query="row_count"
    fi
  fi

  case "$selected_query" in
    row_count)
      case_name="export once repeated row count"
      query_label="row count"
      bcftools_expr="__ROW_COUNT__"
      ;;
    qual_gt_30)
      case_name="export once repeated QUAL > 30"
      query_label="QUAL > 30"
      bcftools_expr="QUAL>30"
      ;;
    filter_pass)
      case_name="export once repeated FILTER == \"PASS\""
      query_label='FILTER == "PASS"'
      bcftools_expr='FILTER="PASS"'
      ;;
    *)
      echo "unsupported VCF_FAST_COLUMNAR_QUERY=$selected_query; expected auto, row_count, qual_gt_30, or filter_pass" >&2
      exit 2
      ;;
  esac

  case_slug="$(slugify "$case_name")-${records}"

  dataset_size="$(file_size_bytes "$dataset")"
  parquet_out="$OUT_DIR/variants-${MODE}-${records}.parquet"
  hyperfine_json="$OUT_DIR/hyperfine-columnar-${MODE}-${records}.json"
  export_command="./target/release/vcf-fast convert $dataset --to parquet -o $parquet_out"
  duckdb_command="$PYTHON benchmark/query_parquet_duckdb.py $parquet_out --query $selected_query --repeats $REPEATED_QUERIES"
  if [[ "$bcftools_expr" == "__ROW_COUNT__" ]]; then
    competitor_command="repeat $REPEATED_QUERIES x: bcftools view -H $dataset | wc -l"
    bcftools_hyperfine_command="bash -c 'for _ in \$(seq 1 \"\$1\"); do bcftools view -H \"\$2\" | wc -l >/dev/null; done' _ $REPEATED_QUERIES $dataset"
    baseline_label="bcftools view row count"
  else
    competitor_command="repeat $REPEATED_QUERIES x: bcftools filter -i '$bcftools_expr' $dataset -Ou | bcftools view -H | wc -l"
    bcftools_hyperfine_command="bash -c 'for _ in \$(seq 1 \"\$1\"); do bcftools filter -i \"\$2\" \"\$3\" -Ou | bcftools view -H | wc -l >/dev/null; done' _ $REPEATED_QUERIES '$bcftools_expr' $dataset"
    baseline_label="bcftools filter count"
  fi

  ./target/release/vcf-fast convert "$dataset" --to parquet -o "$parquet_out"
  duckdb_count="$("$PYTHON" benchmark/query_parquet_duckdb.py "$parquet_out" --query "$selected_query" --repeats 1)"
  bcftools_count="$(repeated_bcftools_count "$dataset" 1 "$bcftools_expr")"
  if [[ "$duckdb_count" != "$bcftools_count" ]]; then
    echo "DuckDB count $duckdb_count did not match bcftools count $bcftools_count for $dataset" >&2
    exit 1
  fi

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --export-json "$hyperfine_json" \
    "./target/release/vcf-fast convert $dataset --to parquet -o $OUT_DIR/variants-${MODE}-${records}.timed.parquet" \
    "$PYTHON benchmark/query_parquet_duckdb.py $parquet_out --query $selected_query --repeats $REPEATED_QUERIES >/dev/null" \
    "$bcftools_hyperfine_command"

  read -r export_mean export_stddev duckdb_mean duckdb_stddev bcftools_mean bcftools_stddev query_speedup amortized_speedup < <(summarize_hyperfine "$hyperfine_json")

  export_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-export-${MODE}-${records}.txt" ./target/release/vcf-fast convert "$dataset" --to parquet -o "$OUT_DIR/variants-${MODE}-${records}.rss.parquet")"
  duckdb_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-duckdb-${MODE}-${records}.txt" "$PYTHON" benchmark/query_parquet_duckdb.py "$parquet_out" --query "$selected_query" --repeats "$REPEATED_QUERIES")"
  if [[ "$bcftools_expr" == "__ROW_COUNT__" ]]; then
    bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${MODE}-${records}.txt" bash -c 'for _ in $(seq 1 "$1"); do bcftools view -H "$2" | wc -l >/dev/null; done' _ "$REPEATED_QUERIES" "$dataset")"
  else
    bcftools_rss="$(measure_peak_rss_kb "$OUT_DIR/rss-bcftools-${MODE}-${records}.txt" bash -c 'for _ in $(seq 1 "$1"); do bcftools filter -i "$2" "$3" -Ou | bcftools view -H | wc -l >/dev/null; done' _ "$REPEATED_QUERIES" "$bcftools_expr" "$dataset")"
  fi
  vps="$(variants_per_second "$records" "$duckdb_mean")"

  claim="correctness matched; repeated Parquet query needs optimization before speed claim"
  if "$PYTHON" - "${amortized_speedup%x}" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) > 1 else 1)
PY
  then
    claim="amortized export-plus-repeated-query workflow measured faster than repeated bcftools scans"
  fi

  export_command_md="$(escape_markdown_table_cell "$export_command")"
  duckdb_command_md="$(escape_markdown_table_cell "$duckdb_command")"
  competitor_command_md="$(escape_markdown_table_cell "$competitor_command")"

  printf '| %s | %s | %s | %s | `%s` | `%s` | `%s` | %s | %s +/- %s | %s +/- %s | %s +/- %s | %s | %s | %s queried variants/sec | export %s / duckdb %s / bcftools %s KB | %s | %s |\n' \
    "$case_name" \
    "$dataset_source" \
    "$dataset_size" \
    "$records" \
    "$export_command_md" \
    "$duckdb_command_md" \
    "$competitor_command_md" \
    "DuckDB $query_label count $duckdb_count matches $baseline_label $bcftools_count" \
    "$export_mean" "$export_stddev" \
    "$duckdb_mean" "$duckdb_stddev" \
    "$bcftools_mean" "$bcftools_stddev" \
    "$query_speedup" \
    "$amortized_speedup" \
    "$vps" \
    "$export_rss" "$duckdb_rss" "$bcftools_rss" \
    "columnar workflow evidence only; not a replacement for native streaming filter" \
    "$claim" >>"$REPORT"
done

{
  echo
  echo "## Raw Artifacts"
  echo
  echo "- Working datasets: \`$DATA_DIR\`"
  echo "- Hyperfine JSON files: \`$OUT_DIR/hyperfine-columnar-*.json\`"
  echo "- Peak RSS files: \`$OUT_DIR/rss-*.txt\`"
} >>"$REPORT"

echo "v1.0 columnar workflow report written to $REPORT"
