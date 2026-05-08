#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="${VCF_FAST_V28_MODE:-smoke}"
OUT_DIR="${VCF_FAST_V28_OUT_DIR:-tests/output/benchmark-results/v28-big-evidence-pass}"
REPORT="${VCF_FAST_V28_REPORT:-$OUT_DIR/v28-big-linux-evidence-pass.md}"
# release gate text: claim matrix must contain no unsupported broad claims.

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"

{
  echo "# VariantFlow v2.8 Big Linux Evidence Pass"
  echo
  echo "This report is the release-gate orchestrator for v3.0 candidate evidence. It combines v2.3 BGZF pipeline, v2.4 .vfi pushdown, v2.5 packed genotype, and v2.6 columnar workflow evidence, then checks the Release gate before any Bioconda, paper, or broad public claim work."
  echo
  echo "## Release gate"
  echo
  echo "- \`make verify\`"
  echo "- \`cargo test --features htslib-static\`"
  echo "- \`cargo clippy --features htslib-static --all-targets -- -D warnings\`"
  echo "- \`make vcftools-parity\`"
  echo "- Claim matrix review for unsupported broad claims"
  echo "- Linux RSS, CPU seconds, and CPU-hour fields present in generated evidence reports"
  echo
  echo "## Evidence Components"
  echo
  echo "| component | target | status | source report | correctness gate | claim decision |"
  echo "| --- | --- | --- | --- | --- | --- |"
  echo "| v2.3 BGZF pipeline | bench-v23-pipeline | pending | tests/output/benchmark-results/v23-bgzf-pipeline | default/native/parallel byte-for-byte and bcftools core records | no broad best-tool claim |"
  echo "| v2.4 .vfi pushdown | bench-v24-index | pending | tests/output/benchmark-results/v24-index-pushdown | indexed output matches default streaming | no broad best-tool claim |"
  echo "| v2.5 packed genotype | bench-v25-genotype | pending | tests/output/benchmark-results/v25-packed-genotype | VCFtools parity | no broad best-tool claim |"
  echo "| v2.6 columnar workflow | bench-v26-columnar | pending | tests/output/benchmark-results/v26-columnar-pushdown | DuckDB rows match normalized VCF/bcftools baselines | no broad best-tool claim |"
} >"$REPORT"

if [[ "$MODE" == "report-only" || "$MODE" == "smoke" ]]; then
  echo "Wrote v2.8 evidence-pass scaffold to $REPORT"
fi

if [[ "$MODE" == "report-only" ]]; then
  exit 0
fi

if [[ "${VCF_FAST_V28_RUN_VERIFY:-0}" == "1" || "$MODE" == "full" ]]; then
  make verify
  cargo test --features htslib-static
  cargo clippy --features htslib-static --all-targets -- -D warnings
  make vcftools-parity
fi

if [[ "$MODE" == "smoke" ]]; then
  VCF_FAST_V23_MODE=smoke VCF_FAST_V23_RUNS=1 make bench-v23-pipeline
  VCF_FAST_V24_DRY_RUN=1 make bench-v24-index
  VCF_FAST_V25_DRY_RUN=1 make bench-v25-genotype
  VCF_FAST_V26_DRY_RUN=1 make bench-v26-columnar
  exit 0
fi

if [[ "$MODE" != "full" ]]; then
  echo "unsupported VCF_FAST_V28_MODE=$MODE; expected smoke, full, or report-only" >&2
  exit 2
fi

make bench-v23-pipeline
make bench-v24-index
make bench-v25-genotype
make bench-v26-columnar

{
  echo
  echo "## Completion Notes"
  echo
  echo "- Big evidence pass completed at $(date -u '+%Y-%m-%dT%H:%M:%SZ')."
  echo "- Next step: update docs/claim-matrix.md only from correctness-matched rows."
} >>"$REPORT"

echo "Wrote v2.8 evidence-pass report to $REPORT"
