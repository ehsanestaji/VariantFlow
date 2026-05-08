# v2.1 Indexed Filter Benchmark

This report measures VariantFlow v2.1 Indexed Filter behavior using BGZF virtual offsets. It compares default native filtering, indexed native filtering, and bcftools filter.

The public mode stages bounded BGZF tiers from the cached 1000 Genomes / IGSR chr22 VCF without writing a plain VCF intermediate. The default expression is `AF > 0.99`, which exercises INFO/AF chunk metadata on real public records.

Rows outside the configured tiers are not yet measured; keep that caveat attached to any claim decision.

## Environment

- sizes: `10000 100000 1000000`
- mode: `public-igsr`
- runs: `3`
- warmup: `1`
- expression: `AF > 0.99`
- bcftools expression: `INFO/AF>0.99`
- index minimum skip rate: `0.80`
- dataset source: `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`
- cached input: `tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`
- bcftools: `bcftools 1.23.1`
- hyperfine: `hyperfine 1.20.0`

## Results

| tier records | index action | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | core records | correctness result | guarded indexed runtime mean +/- stddev | default runtime mean +/- stddev | bcftools runtime mean +/- stddev | speedup | guarded variants/sec | peak RSS | claim decision | caveat |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| 10000 | fell back: VFI skip estimate 0.181 is below required minimum 0.800 | 2 | 1 | 50.0% | 1808 | 2 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.027872s +/- 0.002684s | 0.030882s +/- 0.000612s | 0.279114s +/- 0.001512s | 1.11x vs default; 10.01x vs bcftools | 358783 | indexed 8256 KB; bcftools 9520 KB | claim decision: correctness passed; VFI planner fell back to default native because skip estimate was below threshold | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |
| 100000 | fell back: VFI skip estimate 0.000 is below required minimum 0.800 | 13 | 0 | 0.0% | 0 | 41 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.222513s +/- 0.001195s | 0.234852s +/- 0.011118s | 2.962587s +/- 0.156884s | 1.06x vs default; 13.31x vs bcftools | 449412 | indexed 8288 KB; bcftools 9600 KB | claim decision: correctness passed; VFI planner fell back to default native because skip estimate was below threshold | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |
| 1000000 | fell back: VFI skip estimate 0.017 is below required minimum 0.800 | 123 | 3 | 2.4% | 16960 | 618 | default and indexed byte-for-byte match; indexed and bcftools core records match | 2.612653s +/- 0.097858s | 2.835607s +/- 0.402106s | 29.532228s +/- 0.845144s | 1.09x vs default; 11.30x vs bcftools | 382753 | indexed 8464 KB; bcftools 9600 KB | claim decision: correctness passed; VFI planner fell back to default native because skip estimate was below threshold | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |

## Exact Commands

### Commands for 10000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-10000-bench.json VCF_FAST_INDEX_MIN_SKIP_RATE=0.80 target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz -o /dev/null `

### Commands for 100000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-100000-bench.json VCF_FAST_INDEX_MIN_SKIP_RATE=0.80 target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz -o /dev/null `

### Commands for 1000000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-1000000-bench.json VCF_FAST_INDEX_MIN_SKIP_RATE=0.80 target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz -o /dev/null `
