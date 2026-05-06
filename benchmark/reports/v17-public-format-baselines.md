# v1.7 Public FORMAT And Optional Baselines

This report tracks public FORMAT-heavy and ecosystem baseline evidence. Full
runs stay local and reproducible; CI should use smoke tiers only.

Dataset target: FORMAT-rich public trio/cohort VCF. Default target is the
FORMAT-rich public cohort from ENA ERZ324584 when cached: an Ovis aries
chromosome 19 VCF described by ENA as 453 sheep using GATK and samtools, with
453-sample FORMAT/AD and FORMAT/DP columns. The larger FORMAT-rich WGS trio
from Zenodo remains the next fallback when cached, and the SourceForge 123VCF
NA12878 trio remains the smoke fallback. Override with `VCF_FAST_FORMAT_VCF`
or `VCF_FAST_FORMAT_COHORT_VCF` for another validated FORMAT-rich public
cohort. Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP.

Validated ENA cohort target: `ERZ324584`, `453` samples, `1097167` indexed
records, `2213677122` bytes, MD5 `9dabe9929a8923e62c8808d6fbf15314`.

Repeated local timing uses `hyperfine` when available
(`VCF_FAST_V17_RUNS=3`, `VCF_FAST_V17_WARMUP=1`). Peak RSS is
reported from GNU `/usr/bin/time -v` on Linux or BSD `/usr/bin/time -l` on
macOS.

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|
| public FORMAT-heavy | tests/output/public-data/19.filtered_intersect.vcf.gz | 10000 requested / 10000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-10000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-10000.vcf ` | matched core records | VariantFlow 0.400135s +/- 0.088828s; bcftools 0.876776s +/- 0.175056s; speedup 2.19x | VariantFlow 3976; bcftools 7124 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz; samples=453; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/19.filtered_intersect.vcf.gz | 50000 requested / 50000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-50000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-50000.vcf ` | matched core records | VariantFlow 1.950393s +/- 0.110795s; bcftools 3.707527s +/- 0.633328s; speedup 1.90x | VariantFlow 4004; bcftools 7124 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz; samples=453; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/19.filtered_intersect.vcf.gz | 100000 requested / 100000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-100000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-100000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-100000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-100000.vcf ` | matched core records | VariantFlow 4.188849s +/- 0.262143s; bcftools 11.698374s +/- 5.012201s; speedup 2.79x | VariantFlow 4012; bcftools 7008 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz; samples=453; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/19.filtered_intersect.vcf.gz | 250000 requested / 250000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-250000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-250000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-250000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-250000.vcf ` | matched core records | VariantFlow 10.702888s +/- 1.033992s; bcftools 18.829522s +/- 2.020668s; speedup 1.76x | VariantFlow 4016; bcftools 7036 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz; samples=453; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |

## Optional baselines

- VCFtools: enabled only with `VCF_FAST_ENABLE_VCFTOOLS=1`.
- GATK SelectVariants / VariantFiltration: enabled only with `VCF_FAST_ENABLE_GATK=1`.
- Polars: enabled only with `VCF_FAST_ENABLE_POLARS=1`.
- PyArrow: enabled only with `VCF_FAST_ENABLE_PYARROW=1`.

Optional baseline rows remain `not yet proven` until correctness and runtime
are recorded.
