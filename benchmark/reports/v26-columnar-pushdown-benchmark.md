# VariantFlow v2.6 Columnar Pushdown Benchmark

This scaffold tracks Parquet row-group sizing and DuckDB repeated-query evidence. The goal is workflow-specific: export once, then make repeated analysis faster through row-group metadata and query pushdown over `CHROM`, `FILTER`, `QUAL`, `INFO/DP`, and `INFO/AF`.

## Required Fields

- row-group sizing
- DuckDB version
- export time
- query-only time
- amortized time
- break-even query count
- peak RSS KB
- exact export command
- exact DuckDB command
- exact competitor command
- correctness result
- caveat

## Measured Rows

| dataset | tier | row-group sizing | query | export time | query-only time | amortized time | break-even query count | peak RSS KB | correctness result | claim decision |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| pending | pending | pending | DuckDB | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured |

## Caveat

This is not a claim that Parquet replaces streaming filters. It is a repeated-analysis bridge claim, and rows require matching normalized VCF or `bcftools` baselines.
