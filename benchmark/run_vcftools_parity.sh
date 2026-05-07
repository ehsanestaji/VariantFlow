#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_VCFTOOLS_OUT_DIR:-tests/output/vcftools-parity}"
INPUT="${VCF_FAST_VCFTOOLS_INPUT:-tests/data/popgen_stats.vcf}"
mkdir -p "$OUT_DIR"

if ! command -v vcftools >/dev/null 2>&1; then
  echo "vcftools not found; skipping optional VCFtools parity run."
  echo "Install vcftools and rerun: make vcftools-parity"
  exit 0
fi

cargo build --release

./target/release/variantflow freq "$INPUT" -o "$OUT_DIR/variantflow.frq"
./target/release/variantflow missingness "$INPUT" -o "$OUT_DIR/variantflow-missingness"
./target/release/variantflow hardy "$INPUT" -o "$OUT_DIR/variantflow.hwe"
./target/release/variantflow het "$INPUT" -o "$OUT_DIR/variantflow.het"

vcftools --vcf "$INPUT" --freq --out "$OUT_DIR/vcftools-freq"
vcftools --vcf "$INPUT" --missing-site --missing-indv --out "$OUT_DIR/vcftools-missingness"
vcftools --vcf "$INPUT" --hardy --out "$OUT_DIR/vcftools-hardy"
vcftools --vcf "$INPUT" --het --out "$OUT_DIR/vcftools-het"

cat > "$OUT_DIR/README.md" <<EOF
# Optional VCFtools parity artifacts

Generated from:

- Input: \`$INPUT\`
- VariantFlow: \`./target/release/variantflow\`
- VCFtools: \`$(vcftools --version 2>&1 | head -n 1)\`

This optional harness captures side-by-side outputs for manual/exact normalizer
development. The default repository tests use deterministic fixtures because
\`vcftools\` is not required for CI.
EOF

echo "VCFtools parity artifacts written to $OUT_DIR"
