# VCF-Fast v1.0 Columnar Workflow Evidence Design

## Goal

Prove whether native Parquet export creates a real workflow advantage for repeated analytical queries. The benchmark should keep the native selective filter as the core speed claim, then measure Parquet as the bridge for repeated analysis rather than as a replacement for streaming filters.

## Scope

This slice adds a reproducible benchmark harness, report, and claim-map updates. It does not change the public CLI. The first workflow engine is DuckDB because it can query Parquet directly and is common in local data-engineering workflows. PyArrow and Polars remain later optional workflow baselines.

## Benchmark Design

The harness stages deterministic stress VCF data by default and can stage bounded IGSR chr22 public-heavy data without writing a huge plain IGSR intermediate. It then runs:

- `vcf-fast convert <input> --to parquet -o <variants.parquet>`
- DuckDB repeated queries over the exported Parquet file.
- Matching repeated `bcftools` scans over the original VCF/BGZF input.

The first correctness query is `QUAL > 30`, because VCF-Fast Parquet stores `QUAL` as nullable `FLOAT64` and `bcftools filter -i 'QUAL>30'` is the trusted baseline. A second FILTER/PASS count is available for future report expansion, but the acceptance claim is based on the QUAL count.

## Evidence Rules

The report must separate:

- export cost,
- repeated Parquet query cost,
- repeated `bcftools` scan cost,
- amortized columnar workflow cost,
- correctness count match,
- dataset source and shape,
- caveats.

A speed win is claimable only when the DuckDB count matches the `bcftools` count. If DuckDB is unavailable, the script exits with a clear dependency message instead of inventing evidence.

## v1.1 Handoff

After columnar workflow evidence is in place, v1.1 should start parallel native execution. The next implementation target is native parallel BGZF/filter scheduling, keeping line-preserving output guarantees and using the columnar report to decide where repeated-query workflows should remain Parquet-first.
