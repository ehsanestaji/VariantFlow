# VariantFlow v2.3 BGZF Pipeline Benchmark

This report is the scaffold for measuring native BGZF decompression and predicate evaluation as separable pipeline stages. It records only rows produced by `benchmark/run_v23_bgzf_pipeline_benchmarks.sh`; no measured rows are checked in by default.

## Required Modes

- `forced-single`
- `bgzf-only`
- `predicate-only`
- `combined-pipeline`
- `bcftools filter`

## Measured Rows

| dataset | mode | exact command | wall seconds | peak RSS KB | correctness result | notes |
| --- | --- | --- | ---: | ---: | --- | --- |

## Claim Policy

Runtime claims for v2.3 require measured rows from this harness with `correctness result` showing byte-for-byte agreement across VariantFlow modes and core-record agreement against `bcftools filter`.
