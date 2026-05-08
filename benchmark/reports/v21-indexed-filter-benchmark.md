# v2.1 Indexed Filter Benchmark

This report measures VariantFlow v2.1 Indexed Filter behavior using BGZF virtual offsets. It compares default native filtering, indexed native filtering, and bcftools filter. The default expression is `QUAL > 1000`, which is designed to skip all deterministic stress chunks because generated QUAL values are 0..99.

Rows outside the configured tiers are not yet measured; keep that caveat attached to any claim decision.

## Environment

- sizes: `10000 100000 1000000`
- runs: `3`
- warmup: `1`
- expression: `QUAL > 1000`
- bcftools expression: `QUAL>1000`
- bcftools: `bcftools 1.23.1`
- hyperfine: `hyperfine 1.20.0`

## Results

| tier records | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | core records | correctness result | indexed runtime mean +/- stddev | default runtime mean +/- stddev | bcftools runtime mean +/- stddev | speedup | indexed variants/sec | peak RSS | claim decision | caveat |
| ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| 10000 | 2 | 2 | 100.0% | 10000 | 0 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.012266s +/- 0.007171s | 0.012676s +/- 0.000247s | 0.026049s +/- 0.000875s | 1.03x vs default; 2.12x vs bcftools | 815262 | indexed 7968 KB; bcftools 8000 KB | claim decision: correctness passed; speed claim allowed only for this measured row | synthetic stress BGZF only; public-data caveat and broader predicates are not covered |
| 100000 | 13 | 13 | 100.0% | 100000 | 0 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.008039s +/- 0.000513s | 0.031508s +/- 0.001026s | 0.180669s +/- 0.002144s | 3.92x vs default; 22.47x vs bcftools | 12439358 | indexed 8048 KB; bcftools 8016 KB | claim decision: correctness passed; speed claim allowed only for this measured row | synthetic stress BGZF only; public-data caveat and broader predicates are not covered |
| 1000000 | 122 | 122 | 100.0% | 1000000 | 0 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.030555s +/- 0.000804s | 0.185297s +/- 0.003711s | 1.704888s +/- 0.006717s | 6.06x vs default; 55.80x vs bcftools | 32727868 | indexed 8176 KB; bcftools 8000 KB | claim decision: correctness passed; speed claim allowed only for this measured row | synthetic stress BGZF only; public-data caveat and broader predicates are not covered |

## Exact Commands

### Commands for 10000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz -o tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz.vfi tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-indexed-filter/index-report-10000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- bcftools: `bcftools filter -Ov -i QUAL\>1000 tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-10000.vcf.gz -o /dev/null `

### Commands for 100000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz -o tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz.vfi tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-indexed-filter/index-report-100000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- bcftools: `bcftools filter -Ov -i QUAL\>1000 tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-100000.vcf.gz -o /dev/null `

### Commands for 1000000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz -o tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz.vfi tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-indexed-filter/index-report-1000000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz --where QUAL\ \>\ 1000 -o /dev/null `
- bcftools: `bcftools filter -Ov -i QUAL\>1000 tests/output/benchmark-results/v21-indexed-filter/data/v21-indexed-filter-stress-1000000.vcf.gz -o /dev/null `
