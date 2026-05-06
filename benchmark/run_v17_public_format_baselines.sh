#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v17-public-format-baselines}"
REPORT="${VCF_FAST_V17_REPORT:-benchmark/reports/v17-public-format-baselines.md}"
FORMAT_TRIO_VCF="tests/output/public-data/NA12878.trio.hg19_multianno.vcf.gz"
FORMAT_TRIO_URL="https://sourceforge.net/projects/project123vcf/files/Benchmark_Data/NA12878.trio.hg19_multianno.vcf.gz/download"
FORMAT_COHORT_VCF="${VCF_FAST_FORMAT_COHORT_VCF:-tests/output/public-data/19.filtered_intersect.vcf.gz}"
FORMAT_COHORT_URL="https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz"
FORMAT_COHORT_ENA_ACCESSION="ERZ324584"
FORMAT_COHORT_MD5="9dabe9929a8923e62c8808d6fbf15314"
FORMAT_COHORT_BYTES="2213677122"
FORMAT_COHORT_RECORDS="1097167"
FORMAT_COHORT_SAMPLES="453"
FORMAT_WGS_TRIO_VCF="${VCF_FAST_FORMAT_WGS_TRIO_VCF:-tests/output/public-data/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz}"
FORMAT_WGS_TRIO_URL="https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1"
MAYO_VCF_MINER_PAGE="https://bioinformaticstools.mayo.edu/research/vcf-miner-sample-vcfs/"
MAYO_COHORT_NOTE="Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP."
if [[ -n "${VCF_FAST_FORMAT_VCF:-}" ]]; then
  PUBLIC_DATA="$VCF_FAST_FORMAT_VCF"
  PUBLIC_SOURCE_URL="${VCF_FAST_FORMAT_VCF_URL:-user supplied VCF_FAST_FORMAT_VCF}"
elif [[ -s "$FORMAT_COHORT_VCF" ]]; then
  PUBLIC_DATA="$FORMAT_COHORT_VCF"
  PUBLIC_SOURCE_URL="$FORMAT_COHORT_URL"
elif [[ -s "$FORMAT_WGS_TRIO_VCF" ]]; then
  PUBLIC_DATA="$FORMAT_WGS_TRIO_VCF"
  PUBLIC_SOURCE_URL="$FORMAT_WGS_TRIO_URL"
else
  PUBLIC_DATA="$FORMAT_TRIO_VCF"
  PUBLIC_SOURCE_URL="$FORMAT_TRIO_URL"
fi
TIERS="${VCF_FAST_V17_TIERS:-10000 50000 100000 250000}"
RUNS="${VCF_FAST_V17_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V17_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
FORMAT_EXPR='N_PASS(FORMAT/AD[1] > 10) >= 2'
BCFTOOLS_EXPR='N_PASS(FMT/AD[*:1]>10)>=2'

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
  stream_public_vcf | awk '
    /^##FORMAT=<ID=AD,/ { ad = 1 }
    /^##FORMAT=<ID=DP,/ { dp = 1 }
    END { if (!(ad && dp)) exit 1 }
  '
}

sample_count() {
  local vcf="$1"
  bcftools query -l "$vcf" | wc -l | tr -d ' '
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

write_header() {
  cat >"$REPORT" <<EOF
# v1.7 Public FORMAT And Optional Baselines

This report tracks public FORMAT-heavy and ecosystem baseline evidence. Full
runs stay local and reproducible; CI should use smoke tiers only.

Dataset target: FORMAT-rich public trio/cohort VCF. Default target is the
FORMAT-rich public cohort from ENA ERZ324584 when cached: an Ovis aries
chromosome 19 VCF described by ENA as 453 sheep using GATK and samtools, with
453-sample FORMAT/AD and FORMAT/DP columns. The larger FORMAT-rich WGS trio
from Zenodo remains the next fallback when cached, and the SourceForge 123VCF
NA12878 trio remains the smoke fallback. Override with \`VCF_FAST_FORMAT_VCF\`
or \`VCF_FAST_FORMAT_COHORT_VCF\` for another validated FORMAT-rich public
cohort. $MAYO_COHORT_NOTE

Validated ENA cohort target: \`$FORMAT_COHORT_ENA_ACCESSION\`,
\`$FORMAT_COHORT_SAMPLES\` samples, \`$FORMAT_COHORT_RECORDS\` indexed records,
\`$FORMAT_COHORT_BYTES\` bytes, MD5 \`$FORMAT_COHORT_MD5\`.

Repeated local timing uses \`hyperfine\` when available
(\`VCF_FAST_V17_RUNS=$RUNS\`, \`VCF_FAST_V17_WARMUP=$WARMUP\`). Peak RSS is
reported from GNU \`/usr/bin/time -v\` on Linux or BSD \`/usr/bin/time -l\` on
macOS.

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
  echo "| public FORMAT-heavy | $PUBLIC_DATA | n/a | n/a | n/a | missing public data | n/a | n/a | not yet proven | run benchmark/download_public_data.sh format-ovis453 or set VCF_FAST_FORMAT_VCF to a cached FORMAT-rich cohort |" >>"$REPORT"
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

  build_bounded_subset "$tier" "$subset"
  actual_records=$(bcftools view -H "$subset" | wc -l | tr -d ' ')
  samples=$(sample_count "$subset")

  fast_cmd=(./target/release/variantflow filter "$subset" --where "$FORMAT_EXPR" -o "$fast_out")
  bcftools_cmd=(bcftools filter -i "$BCFTOOLS_EXPR" "$subset" -o "$bcftools_out")
  fast_timed=(./target/release/variantflow filter "$subset" --where "$FORMAT_EXPR" -o "${OUT_DIR}/variantflow-format-${tier}.timed.vcf")
  bcftools_timed=(bcftools filter -i "$BCFTOOLS_EXPR" "$subset" -o "${OUT_DIR}/bcftools-format-${tier}.timed.vcf")

  fast_label="variantflow-format-${tier}"
  bcftools_label="bcftools-format-${tier}"
  fast_rss=$(measure_peak_rss_kb "$fast_label" "${fast_cmd[@]}")
  bcftools_rss=$(measure_peak_rss_kb "$bcftools_label" "${bcftools_cmd[@]}")
  read -r fast_seconds fast_stddev <<<"$(runtime_mean_stddev "${fast_label}" "$(shell_command "${fast_timed[@]}")")"
  read -r bcftools_seconds bcftools_stddev <<<"$(runtime_mean_stddev "${bcftools_label}" "$(shell_command "${bcftools_timed[@]}")")"
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

  echo "| public FORMAT-heavy | $PUBLIC_DATA | $tier requested / $actual_records actual | \`$(shell_command "${fast_cmd[@]}")\` | \`$(shell_command "${bcftools_cmd[@]}")\` | $correctness | VariantFlow ${fast_seconds}s +/- ${fast_stddev}s; bcftools ${bcftools_seconds}s +/- ${bcftools_stddev}s; speedup ${speedup} | VariantFlow ${fast_rss}; bcftools ${bcftools_rss} | $claim | FORMAT-rich public trio/cohort source: $PUBLIC_SOURCE_URL; samples=$samples; expression uses $FORMAT_EXPR; compare against bcftools filter; $MAYO_COHORT_NOTE |" >>"$REPORT"
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
