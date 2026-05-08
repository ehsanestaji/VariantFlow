# VariantFlow v2.8 Big Linux Evidence Pass

This scaffold is the v3.0 release-gate evidence rollup. It should be filled only after Linux repeated runs with RSS and CPU metrics complete for the major performance families.

## Components

| component | target | status | required correctness gate | release relevance |
| --- | --- | --- | --- | --- |
| v2.3 BGZF pipeline | `bench-v23-pipeline` | pending | native modes match byte-for-byte; `bcftools` core records match | compressed public/stress filtering |
| v2.4 .vfi pushdown | `bench-v24-index` | pending | indexed output matches default streaming | high-skip selective filtering |
| v2.5 packed genotype | `bench-v25-genotype` | pending | VCFtools parity | population-genetics memory and speed |
| v2.6 columnar workflow | `bench-v26-columnar` | pending | DuckDB results match normalized VCF or `bcftools` baselines | repeated analytical queries |

## Release gate

- `make verify`
- `cargo test --features htslib-static`
- `cargo clippy --features htslib-static --all-targets -- -D warnings`
- `make vcftools-parity`
- claim matrix contains no unsupported broad claims
- public reports include Linux RSS, CPU seconds, CPU-hour estimates, exact commands, tool versions, and caveats

## Claim Discipline

no broad best-tool claim is supported by this scaffold. VariantFlow claims should remain workflow-specific and report-backed.
