# v1.7 Public FORMAT And Optional Baselines

This report is the tracked placeholder for the next evidence expansion:
public FORMAT-heavy predicates, reproducible Linux RSS, and optional ecosystem
baselines. It intentionally contains no speed win until full local runs append
measured rows.

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|
| public FORMAT-heavy | IGSR chr22 cached public data | pending | `variantflow filter subset.vcf.gz --where 'N_PASS(FORMAT/AD[1] > 10) >= 2' -o out.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:1]>10)>=2' subset.vcf.gz -o out.vcf` | not yet proven | pending | pending | not yet proven | run `make bench-v17` |

## Optional baselines

- VCFtools: later filter/stats comparison when `VCF_FAST_ENABLE_VCFTOOLS=1`.
- GATK SelectVariants / VariantFiltration: heavier Java baseline when
  `VCF_FAST_ENABLE_GATK=1`.
- Polars: columnar workflow baseline when `VCF_FAST_ENABLE_POLARS=1`.
- PyArrow: columnar workflow baseline when `VCF_FAST_ENABLE_PYARROW=1`.

The claim matrix must remain unchanged until correctness and runtime rows are
generated from this harness.
