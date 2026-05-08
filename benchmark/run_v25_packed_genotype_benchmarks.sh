#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TIERS="${VCF_FAST_V25_TIERS:-100000 1000000}"
RUNS="${VCF_FAST_V25_RUNS:-${VCF_FAST_BENCH_RUNS:-3}}"
WARMUP="${VCF_FAST_V25_WARMUP:-${VCF_FAST_BENCH_WARMUP:-1}}"
OUT_DIR="${VCF_FAST_V25_OUT_DIR:-tests/output/benchmark-results/v25-packed-genotype}"
REPORT="${VCF_FAST_V25_REPORT:-$OUT_DIR/v25-packed-genotype-benchmark.md}"
PUBLIC_INPUT="${VCF_FAST_V25_INPUT:-tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz}"
PUBLIC_METADATA="${VCF_FAST_V25_METADATA:-tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt}"

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

{
  echo "# VariantFlow v2.5 Packed Genotype Benchmark"
  echo
  echo "This report tracks the packed diploid biallelic genotype core for VCFtools-style population genetics. It focuses first on LD RSS, then frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, and Weir-Cockerham Fst."
  echo
  echo "## Configuration"
  echo
  echo "- Tiers: \`$TIERS\`"
  echo "- Runs: \`$RUNS\`"
  echo "- Warmup: \`$WARMUP\`"
  echo "- Scope: supported diploid biallelic rows"
  echo
  echo "## Measured Rows"
  echo
  echo "| workflow | tier | record count | sample count | runtime mean/stddev | speedup | samples/sec | peak RSS KB | CPU seconds | CPU-hour estimate | exact VariantFlow command | exact VCFtools command | correctness result | caveat |"
  echo "| --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  echo "| LD RSS | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity must pass before claim updates |"
  echo "| frequency | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| missingness | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| HWE | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| heterozygosity | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| site pi | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| window pi | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| Tajima's D | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo "| Weir-Cockerham Fst | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |"
  echo
  echo "## Execution"
  echo
  echo "The benchmark first runs \`make vcftools-parity\`, then delegates to \`bench-vcftools-true-popgen\` with \`VCF_FAST_V17_TRUE_POP_TIERS\` set from \`VCF_FAST_V25_TIERS\`. Generated rows under \`tests/output\` should be copied into tracked claim reports only after normalized correctness passes."
} >"$REPORT"

if [[ "${VCF_FAST_V25_DRY_RUN:-0}" == "1" ]]; then
  echo "Dry run: wrote $REPORT"
  exit 0
fi

make vcftools-parity

VCF_FAST_V17_TRUE_POP_TIERS="$TIERS" \
VCF_FAST_V17_TRUE_POP_RUNS="$RUNS" \
VCF_FAST_V17_TRUE_POP_WARMUP="$WARMUP" \
VCF_FAST_V17_TRUE_POP_INPUT="$PUBLIC_INPUT" \
VCF_FAST_V17_TRUE_POP_METADATA="$PUBLIC_METADATA" \
VCF_FAST_V17_TRUE_POP_REPORT="$OUT_DIR/true-popgen-report.md" \
make bench-vcftools-true-popgen

{
  echo
  echo "## Source Evidence"
  echo
  echo "- True population source report: \`$OUT_DIR/true-popgen-report.md\`"
  echo "- Required correctness workflows: frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, and Weir-Cockerham Fst."
} >>"$REPORT"

echo "Wrote v2.5 packed genotype report to $REPORT"
