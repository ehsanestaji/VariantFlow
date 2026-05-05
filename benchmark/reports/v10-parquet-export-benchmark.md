# VCF-Fast v1.0 Parquet Export Benchmark

## Status

This report tracks the first v1.0 native Parquet export slice. Correctness is covered by integration tests that read the produced Parquet through Arrow and verify schema, row count, nulls, and preserved AF strings. Runtime rows compare native Parquet export against native TSV export and `bcftools query` TSV projection; they are not broad Parquet workflow claims yet.

## Run Configuration

- Generated: 2026-05-05T17:42:34Z
- Dataset source: deterministic stress data from `benchmark/generate_stress_vcf.sh`
- Dataset shape: stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD
- Record tiers: `10000 100000`
- Repeated runs: `3`
- Warmup runs: `1`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1

## Measured Export Cases

| case | dataset size bytes | record count | exact Parquet command | exact TSV command | exact competitor command | correctness result | parquet mean/stddev | tsv mean/stddev | bcftools mean/stddev | TSV/parquet ratio | bcftools/parquet ratio | variants/sec parquet/tsv/bcftools | peak RSS parquet/tsv/bcftools | caveat | claim decision |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| Stress VCF selected-column export | 8040115 | 10000 | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-parquet/data/stress-10000.vcf --to parquet -o tests/output/benchmark-results/v10-parquet/fast-convert-parquet-10000.parquet` | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-parquet/data/stress-10000.vcf --to tsv -o tests/output/benchmark-results/v10-parquet/fast-convert-tsv-10000.tsv` | `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' tests/output/benchmark-results/v10-parquet/data/stress-10000.vcf > tests/output/benchmark-results/v10-parquet/bcftools-query-tsv-10000.tsv` | Parquet schema/null semantics verified by integration tests; TSV and bcftools row counts match input records | 0.009647s +/- 0.000097s | 0.007987s +/- 0.000134s | 0.018751s +/- 0.000370s | 0.83x | 1.94x | 1036592 / 1252035 / 533305 | n/a / n/a / n/a KB | synthetic stress only; no DuckDB/Polars workflow benchmark yet | measured faster than bcftools query on this projection |
| Stress VCF selected-column export | 80473430 | 100000 | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-parquet/data/stress-100000.vcf --to parquet -o tests/output/benchmark-results/v10-parquet/fast-convert-parquet-100000.parquet` | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-parquet/data/stress-100000.vcf --to tsv -o tests/output/benchmark-results/v10-parquet/fast-convert-tsv-100000.tsv` | `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' tests/output/benchmark-results/v10-parquet/data/stress-100000.vcf > tests/output/benchmark-results/v10-parquet/bcftools-query-tsv-100000.tsv` | Parquet schema/null semantics verified by integration tests; TSV and bcftools row counts match input records | 0.067572s +/- 0.001049s | 0.051085s +/- 0.000906s | 0.130492s +/- 0.000739s | 0.76x | 1.93x | 1479903 / 1957522 / 766331 | n/a / n/a / n/a KB | synthetic stress only; no DuckDB/Polars workflow benchmark yet | measured faster than bcftools query on this projection |

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v10-parquet/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v10-parquet/hyperfine-*.json`
- Peak RSS files: `tests/output/benchmark-results/v10-parquet/rss-*.txt`
