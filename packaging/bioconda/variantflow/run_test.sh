#!/usr/bin/env bash
set -euo pipefail

variantflow --version
vcf-fast --version

cat > example.vcf <<'VCF'
##fileformat=VCFv4.3
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total depth">
##INFO=<ID=AF,Number=A,Type=Float,Description="Allele frequency">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
1	100	rsLow	A	G	10	PASS	DP=8;AF=0.01
1	200	rsPass	C	T	35	PASS	DP=12;AF=0.03
VCF

variantflow filter example.vcf --where "QUAL > 30" -o filtered.vcf
grep -q "rsPass" filtered.vcf
! grep -q "rsLow" filtered.vcf

variantflow convert example.vcf --to tsv -o variants.tsv
grep -q "CHROM" variants.tsv
grep -q "rsPass" variants.tsv
