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
- dataset source: `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`
- cached input: `tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`
- bcftools: `bcftools 1.23.1`
- hyperfine: `hyperfine 1.20.0`

## Results

| tier records | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | core records | correctness result | indexed runtime mean +/- stddev | default runtime mean +/- stddev | bcftools runtime mean +/- stddev | speedup | indexed variants/sec | peak RSS | claim decision | caveat |
| ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |
| 10000 | 2 | 1 | 50.0% | 1808 | 2 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.079213s +/- 0.000632s | 0.037918s +/- 0.001887s | 0.292636s +/- 0.003574s | 0.48x vs default; 3.69x vs bcftools | 126242 | indexed 8352 KB; bcftools 9568 KB | claim decision: correctness passed; indexed beats bcftools but is slower than default native on this measured row | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |
| 100000 | 13 | 0 | 0.0% | 0 | 41 | default and indexed byte-for-byte match; indexed and bcftools core records match | 0.906680s +/- 0.019716s | 0.276206s +/- 0.010572s | 2.833910s +/- 0.031681s | 0.30x vs default; 3.13x vs bcftools | 110292 | indexed 8704 KB; bcftools 9568 KB | claim decision: correctness passed; indexed beats bcftools but is slower than default native on this measured row | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |
| 1000000 | 123 | 3 | 2.4% | 16960 | 618 | default and indexed byte-for-byte match; indexed and bcftools core records match | 8.676507s +/- 0.038022s | 2.446517s +/- 0.091950s | 28.422484s +/- 0.534574s | 0.28x vs default; 3.28x vs bcftools | 115254 | indexed 8896 KB; bcftools 9568 KB | claim decision: correctness passed; indexed beats bcftools but is slower than default native on this measured row | bounded public IGSR chr22 BGZF tiers; AF predicate only; broader predicates and full-chromosome public rows are not covered |

## Exact Commands

### Commands for 10000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-10000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-10000.vcf.gz -o /dev/null `

### Commands for 100000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-100000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-100000.vcf.gz -o /dev/null `

### Commands for 1000000

- index: `target/release/variantflow index tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz -o tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi `
- default: `trap 'mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi.bench-disabled tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi 2>/dev/null || true' EXIT; mv tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz.vfi.bench-disabled; target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- indexed: `VCF_FAST_INDEX_REPORT=tests/output/benchmark-results/v21-public-indexed-filter/index-report-1000000-bench.json target/release/variantflow filter tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz --where AF\ \>\ 0.99 -o /dev/null `
- bcftools: `bcftools filter -Ov -i INFO/AF\>0.99 tests/output/benchmark-results/v21-public-indexed-filter/data/v21-indexed-filter-public-igsr-1000000.vcf.gz -o /dev/null `
