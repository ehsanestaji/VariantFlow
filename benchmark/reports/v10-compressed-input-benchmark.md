# VCF-Fast v1.0 Compressed Input Benchmark

## Status

This report tracks the first v1.0 parallel-compressed-input slice. Runtime claims are limited to rows where threaded native BGZF output matches the `bcftools filter` core-record baseline.

## Run Configuration

- Generated: 2026-05-05T17:27:31Z
- Dataset source: tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz
- Region: `chr22:1-20000000`
- Record tiers: `10000 100000 1000000`
- Native BGZF threads: `4`
- Repeated runs: `3`
- Warmup runs: `1`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1

## Measured Compressed Input Cases

| case | dataset size bytes | record count | exact default VCF-Fast command | exact threaded VCF-Fast command | exact competitor command | correctness result | default mean/stddev | threaded mean/stddev | bcftools mean/stddev | threaded vs default | threaded vs bcftools | variants/sec default/threaded/bcftools | peak RSS default/threaded/bcftools | caveat | claim decision |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| IGSR bounded BGZF QUAL filter | 3382499 | 10000 | `env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-10000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/default-qual-10000.vcf` | `VCF_FAST_NATIVE_BGZF_THREADS=4 ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-10000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/threaded-qual-10000.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/v10-compressed/data/public-heavy-10000.vcf.gz -o tests/output/benchmark-results/v10-compressed/bcftools-qual-10000.vcf` | default and threaded VCF-Fast match bcftools filtered core records | 0.054145s +/- 0.004178s | 0.029207s +/- 0.002337s | 0.289237s +/- 0.004237s | 1.85x | 9.90x | 184689 / 342384 / 34574 | n/a / n/a / n/a KB | bounded chr22 BGZF subset; ordinary gzip is still single-thread fallback | measured faster than bcftools on this BGZF input case |
| IGSR bounded BGZF QUAL filter | 34792224 | 100000 | `env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-100000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/default-qual-100000.vcf` | `VCF_FAST_NATIVE_BGZF_THREADS=4 ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-100000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/threaded-qual-100000.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/v10-compressed/data/public-heavy-100000.vcf.gz -o tests/output/benchmark-results/v10-compressed/bcftools-qual-100000.vcf` | default and threaded VCF-Fast match bcftools filtered core records | 0.484728s +/- 0.006790s | 0.241887s +/- 0.001051s | 2.870070s +/- 0.052121s | 2.00x | 11.87x | 206301 / 413416 / 34842 | n/a / n/a / n/a KB | bounded chr22 BGZF subset; ordinary gzip is still single-thread fallback | measured faster than bcftools on this BGZF input case |
| IGSR bounded BGZF QUAL filter | 75203313 | 1000000 | `env -u VCF_FAST_NATIVE_BGZF_THREADS ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-1000000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/default-qual-1000000.vcf` | `VCF_FAST_NATIVE_BGZF_THREADS=4 ./target/release/vcf-fast filter tests/output/benchmark-results/v10-compressed/data/public-heavy-1000000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v10-compressed/threaded-qual-1000000.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/v10-compressed/data/public-heavy-1000000.vcf.gz -o tests/output/benchmark-results/v10-compressed/bcftools-qual-1000000.vcf` | default and threaded VCF-Fast match bcftools filtered core records | 0.961004s +/- 0.016880s | 0.502929s +/- 0.026788s | 5.785532s +/- 0.524869s | 1.91x | 11.50x | 1040578 / 1988352 / 172845 | n/a / n/a / n/a KB | bounded chr22 BGZF subset; ordinary gzip is still single-thread fallback | measured faster than bcftools on this BGZF input case |

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v10-compressed/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v10-compressed/hyperfine-*.json`
- Equivalence diffs: `tests/output/benchmark-results/v10-compressed/equivalence-*.diff`
- Peak RSS files: `tests/output/benchmark-results/v10-compressed/rss-*.txt`
