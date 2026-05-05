# VCF-Fast v1.1 Parallel Native Filter Benchmark

## Status

This report tracks opt-in parallel native predicate evaluation. The implementation keeps line-preserving output by evaluating bounded batches in parallel and writing accepted original records in input order.

## Run Configuration

- Generated: 2026-05-05T19:32:39Z
- Dataset source: deterministic stress data from `benchmark/generate_stress_vcf.sh`
- Dataset shape: stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD
- Record tiers: `10000 100000`
- Native filter threads: `4`
- Native filter batch records: `4096`
- Repeated runs: `3`
- Warmup runs: `1`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1

## Measured Parallel Filter Cases

| case | dataset source | dataset size bytes | record count | exact default command | exact parallel command | exact competitor command | correctness result | default mean/stddev | parallel mean/stddev | bcftools mean/stddev | parallel vs default | parallel vs bcftools | variants/sec default/parallel/bcftools | peak RSS default/parallel/bcftools | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| Stress ANY FORMAT/AD filter | deterministic stress VCF | 8040115 | 10000 | `env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v11-parallel-filter/data/stress-10000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v11-parallel-filter/default-any-format-ad-10000.vcf` | `VCF_FAST_NATIVE_FILTER_THREADS=4 VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=4096 ./target/release/vcf-fast filter tests/output/benchmark-results/v11-parallel-filter/data/stress-10000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v11-parallel-filter/parallel-any-format-ad-10000.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' tests/output/benchmark-results/v11-parallel-filter/data/stress-10000.vcf -o tests/output/benchmark-results/v11-parallel-filter/bcftools-any-format-ad-10000.vcf` | parallel native matches default native byte-for-byte and matches default native and bcftools filtered core records; line-preserving output retained | 0.014062s +/- 0.000309s | 0.011193s +/- 0.000463s | 0.032416s +/- 0.000037s | 1.26x | 2.90x | 711136 / 893416 / 308490 | n/a / n/a / n/a KB | synthetic stress CPU-heavy expression only; I/O-bound filters may not benefit | parallel native measured faster than default native on this CPU-heavy expression |
| Stress ANY FORMAT/AD filter | deterministic stress VCF | 80473430 | 100000 | `env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v11-parallel-filter/data/stress-100000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v11-parallel-filter/default-any-format-ad-100000.vcf` | `VCF_FAST_NATIVE_FILTER_THREADS=4 VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=4096 ./target/release/vcf-fast filter tests/output/benchmark-results/v11-parallel-filter/data/stress-100000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v11-parallel-filter/parallel-any-format-ad-100000.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' tests/output/benchmark-results/v11-parallel-filter/data/stress-100000.vcf -o tests/output/benchmark-results/v11-parallel-filter/bcftools-any-format-ad-100000.vcf` | parallel native matches default native byte-for-byte and matches default native and bcftools filtered core records; line-preserving output retained | 0.086631s +/- 0.003326s | 0.044264s +/- 0.002049s | 0.216597s +/- 0.004603s | 1.96x | 4.89x | 1154321 / 2259172 / 461687 | n/a / n/a / n/a KB | synthetic stress CPU-heavy expression only; I/O-bound filters may not benefit | parallel native measured faster than default native on this CPU-heavy expression |

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v11-parallel-filter/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v11-parallel-filter/hyperfine-*.json`
- Equivalence diffs: `tests/output/benchmark-results/v11-parallel-filter/equivalence-*.diff`
- Peak RSS files: `tests/output/benchmark-results/v11-parallel-filter/rss-*.txt`
