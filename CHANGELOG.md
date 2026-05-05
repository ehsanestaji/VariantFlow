# Changelog

All notable VCF-Fast changes are tracked here. Claims remain evidence-bound: performance entries refer to benchmark reports, not broad superiority.

## v1.3.0 - Release Hardening

- Added versioned CLI output through `vcf-fast --version`.
- Added release/install guidance for source builds, Docker, benchmark prerequisites, public data, and htslib feature builds.
- Added a generated public benchmark table at `docs/public-benchmark-table.md`.
- Added release workflow scaffolding for macOS and Linux binaries.

## v1.2 - Public Parallel And Workflow Expansion

- Published public-heavy IGSR BGZF filter evidence for default native, predicate-parallel native, threaded BGZF input, combined threading, and `bcftools filter`.
- Added 1M stress parallel evidence for `ANY(FORMAT/AD > 80)`.
- Expanded DuckDB/Parquet workflow checks to `QUAL`, `INFO/DP`, `FILTER`, and grouped `CHROM,FILTER` queries.

## v1.1 - Parallel Native Execution

- Added opt-in native predicate parallelism through `VCF_FAST_NATIVE_FILTER_THREADS`.
- Preserved ordered, line-preserving output while evaluating bounded record batches in parallel.
- Added deterministic stress evidence for CPU-heavy FORMAT aggregate predicates.

## v1.0 - Parallel And Columnar Foundations

- Added opt-in threaded native BGZF input.
- Added native selected-column Parquet export.
- Added DuckDB export-once, query-many workflow evidence.

## v0.9 - Expression Parity

- Added arbitrary `INFO/<KEY>` predicates.
- Added selected-sample `FORMAT/<KEY>` predicates.
- Added native `ANY(FORMAT/<KEY>)` and `ALL(FORMAT/<KEY>)` aggregate predicates.

## v0.8 - Core Efficiency

- Moved native filter and stats paths onto borrowed byte-slice record views.
- Shared byte-based INFO scanning across filter, TSV conversion, and stats.
- Added post-surgery stress and public-heavy evidence.

## v0.7 - Heavy Run And Htslib Optimization

- Avoided giant plain IGSR intermediates in heavy benchmark staging.
- Optimized htslib-backed TSV/stats paths where practical.
- Published compatibility bottleneck notes and measured caveats.

## v0.6 - Public Whole-Cohort Evidence

- Added tiered local public benchmark modes for GIAB and IGSR.
- Added repeated measurements, correctness checks, throughput, and RSS fields.
- Added a contribution claim matrix grounded in `bcftools` and HTSlib comparisons.

## v0.5 - Compatibility Proof

- Added optional htslib-backed paths for BCF input, indexed region reads, and BGZF output.
- Kept the Rust-native line-preserving path as the default engine.

## v0.4 - FORMAT-Aware Filtering

- Added selected-sample `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` filtering.
- Added correctness checks against `bcftools filter`.

## v0.3 - Stress And Speed

- Added deterministic stress VCF generation with many unused INFO/FORMAT/sample fields.
- Added stress filter, TSV, and stats benchmark coverage.

## v0.2 - Public Benchmark Expansion

- Added GIAB/IGSR public-data download and benchmark scaffolding.
- Added repeated benchmark reporting fields and competitor command tracking.

## v0.1 - Evidence Baseline

- Added the Rust `vcf-fast` CLI.
- Added line-preserving native streaming `filter`.
- Added `stats`, `diff`, TSV conversion, Docker, Makefile, fixtures, and synthetic benchmark scaffolding.
