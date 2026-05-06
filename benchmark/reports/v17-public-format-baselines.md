# v1.7 Public FORMAT And Optional Baselines

This report tracks public FORMAT-heavy and ecosystem baseline evidence. Full
runs stay local and reproducible; CI should use smoke tiers only.

Dataset target: FORMAT-rich public trio/cohort VCF. Default target is the
larger FORMAT-rich WGS trio from Zenodo when cached because it declares
FORMAT/AD and FORMAT/DP; otherwise the SourceForge 123VCF NA12878 trio remains
the smoke fallback. Override with `VCF_FAST_FORMAT_VCF` for larger public
cohorts. Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP.

Repeated local timing uses `hyperfine` when available
(`VCF_FAST_V17_RUNS=3`, `VCF_FAST_V17_WARMUP=1`). Peak RSS is
reported from GNU `/usr/bin/time -v` on Linux or BSD `/usr/bin/time -l` on
macOS.

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|
| public FORMAT-heavy | tests/output/public-data/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz | 10000 requested / 10000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-10000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-10000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-10000.vcf ` | matched core records | VariantFlow 0.009183s +/- 0.000481s; bcftools 0.027607s +/- 0.000433s; speedup 3.01x | VariantFlow 7766016; bcftools 8781824 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1; samples=3; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz | 50000 requested / 50000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-50000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-50000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-50000.vcf ` | matched core records | VariantFlow 0.020870s +/- 0.000777s; bcftools 0.105255s +/- 0.000796s; speedup 5.04x | VariantFlow 7815168; bcftools 8945664 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1; samples=3; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz | 100000 requested / 100000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-100000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-100000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-100000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-100000.vcf ` | matched core records | VariantFlow 0.033109s +/- 0.000565s; bcftools 0.195015s +/- 0.013397s; speedup 5.89x | VariantFlow 7798784; bcftools 9076736 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1; samples=3; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |
| public FORMAT-heavy | tests/output/public-data/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz | 250000 requested / 250000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v17-public-format-baselines/format-public-250000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o tests/output/benchmark-results/v17-public-format-baselines/variantflow-format-250000.vcf ` | `bcftools filter -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/benchmark-results/v17-public-format-baselines/format-public-250000.vcf.gz -o tests/output/benchmark-results/v17-public-format-baselines/bcftools-format-250000.vcf ` | matched core records | VariantFlow 0.088727s +/- 0.002125s; bcftools 0.430581s +/- 0.035031s; speedup 4.85x | VariantFlow 7766016; bcftools 9584640 | measured faster on this public FORMAT-rich tier | FORMAT-rich public trio/cohort source: https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1; samples=3; expression uses N_PASS(FORMAT/AD[1] > 10) >= 2; compare against bcftools filter; Mayo VCF-Miner lists 1KG chr22 benchmark VCFs with 629 samples; use VCF_FAST_FORMAT_VCF after caching and validating FORMAT/AD or FORMAT/DP. |

## Optional baselines

- VCFtools: enabled only with `VCF_FAST_ENABLE_VCFTOOLS=1`.
- GATK SelectVariants / VariantFiltration: enabled only with `VCF_FAST_ENABLE_GATK=1`.
- Polars: enabled only with `VCF_FAST_ENABLE_POLARS=1`.
- PyArrow: enabled only with `VCF_FAST_ENABLE_PYARROW=1`.

Optional baseline rows remain `not yet proven` until correctness and runtime
are recorded.
