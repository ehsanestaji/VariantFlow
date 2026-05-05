# VCF-Fast v1.0 Columnar Workflow Benchmark

## Status

This report tests the Parquet workflow claim: export once, then run repeated queries through DuckDB. It compares repeated DuckDB queries over VCF-Fast Parquet output against repeated `bcftools` scans over the original VCF/BGZF input. It does not replace the native selective filter claim.

## Run Configuration

- Generated: 2026-05-05T19:13:21Z
- Mode: `public-heavy`
- Dataset source: deterministic stress data or bounded IGSR public-heavy data
- Dataset source URL: see `benchmark/download_public_data.sh` for the pinned IGSR source when `public-heavy` is used
- Public-heavy region: `chr22:1-20000000`
- Record tiers: `10000 100000`
- Repeated queries: `5`
- Query selector: `auto`
- Repeated runs: `3`
- Warmup runs: `1`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1
- DuckDB: 1.5.2

## Measured Workflow Cases

| case | dataset source | dataset size bytes | record count | exact export command | exact DuckDB command | exact competitor command | correctness result | export mean/stddev | DuckDB repeated query mean/stddev | bcftools repeated scan mean/stddev | query-only speedup | amortized speedup | variants/sec | peak RSS | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| export once repeated row count | bounded IGSR chr22 public-heavy BGZF | 3382499 | 10000 | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-columnar-workflow/data/public-heavy-10000.vcf.gz --to parquet -o tests/output/benchmark-results/v10-columnar-workflow/variants-public-heavy-10000.parquet` | `tests/output/benchmark-results/duckdb-venv/bin/python benchmark/query_parquet_duckdb.py tests/output/benchmark-results/v10-columnar-workflow/variants-public-heavy-10000.parquet --query row_count --repeats 5` | `repeat 5 x: bcftools view -H tests/output/benchmark-results/v10-columnar-workflow/data/public-heavy-10000.vcf.gz \| wc -l` | DuckDB row count count 10000 matches bcftools view row count 10000 | 0.056453s +/- 0.000809s | 0.059449s +/- 0.000631s | 2.777485s +/- 0.017712s | 46.72x | 23.96x | 168211 queried variants/sec | export n/a / duckdb n/a / bcftools n/a KB | columnar workflow evidence only; not a replacement for native streaming filter | amortized export-plus-repeated-query workflow measured faster than repeated bcftools scans |
| export once repeated row count | bounded IGSR chr22 public-heavy BGZF | 34792224 | 100000 | `./target/release/vcf-fast convert tests/output/benchmark-results/v10-columnar-workflow/data/public-heavy-100000.vcf.gz --to parquet -o tests/output/benchmark-results/v10-columnar-workflow/variants-public-heavy-100000.parquet` | `tests/output/benchmark-results/duckdb-venv/bin/python benchmark/query_parquet_duckdb.py tests/output/benchmark-results/v10-columnar-workflow/variants-public-heavy-100000.parquet --query row_count --repeats 5` | `repeat 5 x: bcftools view -H tests/output/benchmark-results/v10-columnar-workflow/data/public-heavy-100000.vcf.gz \| wc -l` | DuckDB row count count 100000 matches bcftools view row count 100000 | 0.509263s +/- 0.000983s | 0.059710s +/- 0.000958s | 27.569380s +/- 0.193808s | 461.72x | 48.45x | 1674761 queried variants/sec | export n/a / duckdb n/a / bcftools n/a KB | columnar workflow evidence only; not a replacement for native streaming filter | amortized export-plus-repeated-query workflow measured faster than repeated bcftools scans |

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v10-columnar-workflow/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v10-columnar-workflow/hyperfine-columnar-*.json`
- Peak RSS files: `tests/output/benchmark-results/v10-columnar-workflow/rss-*.txt`
