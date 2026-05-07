#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_VCFTOOLS_OUT_DIR:-tests/output/vcftools-parity}"
INPUT="${VCF_FAST_VCFTOOLS_INPUT:-tests/data/popgen_stats.vcf}"
POP1="${VCF_FAST_VCFTOOLS_POP1:-tests/data/popgen_pop1.txt}"
POP2="${VCF_FAST_VCFTOOLS_POP2:-tests/data/popgen_pop2.txt}"
WINDOW_SIZE="${VCF_FAST_VCFTOOLS_WINDOW_SIZE:-200}"
LD_WINDOW_BP="${VCF_FAST_VCFTOOLS_LD_WINDOW_BP:-500}"
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
./target/release/variantflow pi "$INPUT" -o "$OUT_DIR/variantflow.sites.pi"
./target/release/variantflow pi "$INPUT" --window-size "$WINDOW_SIZE" -o "$OUT_DIR/variantflow.windowed.pi"
./target/release/variantflow tajima-d "$INPUT" --window-size "$WINDOW_SIZE" -o "$OUT_DIR/variantflow.Tajima.D"
./target/release/variantflow ld "$INPUT" --max-distance "$LD_WINDOW_BP" -o "$OUT_DIR/variantflow.geno.ld"
if ./target/release/variantflow fst --help 2>&1 | grep -q -- '--estimator'; then
  ./target/release/variantflow fst "$INPUT" --pop "$POP1" --pop "$POP2" --estimator weir-cockerham -o "$OUT_DIR/variantflow.weir.fst"
else
  echo "TODO: VariantFlow fst --estimator weir-cockerham is unavailable; writing current fst output for expected parity failure."
  ./target/release/variantflow fst "$INPUT" --pop "$POP1" --pop "$POP2" -o "$OUT_DIR/variantflow.weir.fst"
fi

vcftools --vcf "$INPUT" --freq --out "$OUT_DIR/vcftools-freq"
vcftools --vcf "$INPUT" --missing-site --out "$OUT_DIR/vcftools-missing-site"
vcftools --vcf "$INPUT" --missing-indv --out "$OUT_DIR/vcftools-missing-indv"
vcftools --vcf "$INPUT" --hardy --out "$OUT_DIR/vcftools-hardy"
vcftools --vcf "$INPUT" --het --out "$OUT_DIR/vcftools-het"
vcftools --vcf "$INPUT" --site-pi --out "$OUT_DIR/vcftools-pi"
vcftools --vcf "$INPUT" --window-pi "$WINDOW_SIZE" --out "$OUT_DIR/vcftools-window-pi"
vcftools --vcf "$INPUT" --TajimaD "$WINDOW_SIZE" --out "$OUT_DIR/vcftools-tajima-d"
vcftools --vcf "$INPUT" --geno-r2 --ld-window-bp "$LD_WINDOW_BP" --out "$OUT_DIR/vcftools-ld"
vcftools --vcf "$INPUT" --weir-fst-pop "$POP1" --weir-fst-pop "$POP2" --out "$OUT_DIR/vcftools-weir-fst"

python3 benchmark/check_vcftools_parity.py "$OUT_DIR"

cat > "$OUT_DIR/README.md" <<EOF
# Optional VCFtools parity artifacts

Generated from:

- Input: \`$INPUT\`
- Populations: \`$POP1\`, \`$POP2\`
- Window size: \`$WINDOW_SIZE\`
- LD window bp: \`$LD_WINDOW_BP\`
- VariantFlow: \`./target/release/variantflow\`
- VCFtools: \`$(vcftools --version 2>&1 | head -n 1)\`

This optional harness captures side-by-side outputs and checks normalized parity
for frequency, site/individual missingness, HWE observed/expected/chi-square,
heterozygosity, nucleotide diversity, Tajima's D, LD, and Weir-Cockerham Fst.
If VariantFlow lacks \`fst --estimator weir-cockerham\`, the harness records the
current Fst output at \`variantflow.weir.fst\` so the checker fails at the Fst
estimator parity point.
EOF

echo "VCFtools parity artifacts written to $OUT_DIR"
