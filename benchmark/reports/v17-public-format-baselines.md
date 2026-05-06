# v1.7 Public FORMAT And Optional Baselines

This report tracks public FORMAT-heavy and ecosystem baseline evidence. Full
runs stay local and reproducible; CI should use smoke tiers only.

Dataset target: FORMAT-rich public trio/cohort VCF. Default target is the
SourceForge 123VCF NA12878 trio benchmark because it declares FORMAT/AD and
FORMAT/DP; override with `VCF_FAST_FORMAT_VCF` for larger public cohorts.

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|
| public FORMAT-heavy | tests/output/public-data/NA12878.trio.hg19_multianno.vcf.gz | 10000 | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-10000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-10000.vcf ` | matched core records | VariantFlow 0.34s; bcftools 0.23s; speedup 0.68x | VariantFlow 7929856; bcftools 10469376 | correctness matched; optimization needed before claiming speed win | FORMAT-rich public trio/cohort source: https://sourceforge.net/projects/project123vcf/files/Benchmark_Data/NA12878.trio.hg19_multianno.vcf.gz/download; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter |
| public FORMAT-heavy | tests/output/public-data/NA12878.trio.hg19_multianno.vcf.gz | 50000 | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-50000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-50000.vcf ` | matched core records | VariantFlow 0.27s; bcftools 1.57s; speedup 5.81x | VariantFlow 7962624; bcftools 12304384 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://sourceforge.net/projects/project123vcf/files/Benchmark_Data/NA12878.trio.hg19_multianno.vcf.gz/download; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter |

## Optional baselines

- VCFtools: enabled only with `VCF_FAST_ENABLE_VCFTOOLS=1`.
- GATK SelectVariants / VariantFiltration: enabled only with `VCF_FAST_ENABLE_GATK=1`.
- Polars: enabled only with `VCF_FAST_ENABLE_POLARS=1`.
- PyArrow: enabled only with `VCF_FAST_ENABLE_PYARROW=1`.

Optional baseline rows remain `not yet proven` until correctness and runtime
are recorded.
