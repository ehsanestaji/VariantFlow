# Changelog

All notable VCF-Fast changes are tracked here. Claims remain evidence-bound: performance entries refer to benchmark reports, not broad superiority.

## v1.5.0 - Unreleased

- Accepted `VariantFlow` as the professional public project name.
- Added `variantflow` as the primary CLI binary while keeping `vcf-fast` as a compatibility alias.
- Added `docs/rename-plan.md` and updated release TODOs for Bioconda packaging under `variantflow`.
- Added a local Bioconda recipe scaffold, packaging checker, and `docs/bioconda-packaging.md` with current release blockers.
- Added `make release-candidate-check`, Bioinformatics workflow docs, Snakemake/Nextflow examples, and `docs/claim-matrix.md`.
- Strengthened the Bioinformatics Application Note LaTeX draft around selective execution, public evidence, columnar workflows, and caveats.
- Added vector-index predicates such as `INFO/AF[1]` and `FORMAT/AD[1]`.
- Added `N_PASS(FORMAT/<KEY>[i] op value)` aggregate filtering for cohort/sample-heavy predicates.
- Added diff modes and key modes through `variantflow diff --mode ... --key ...`.
- Added `filter_counts` to native stats JSON summaries.
- Added the v1.7 public FORMAT-heavy and optional ecosystem-baseline harness scaffold.

## v1.4.0 - Public Parallel Scale

- Auto-enabled native BGZF reader workers for BGZF `.vcf.gz` inputs when `VCF_FAST_NATIVE_BGZF_THREADS` is unset.
- Added `VCF_FAST_NATIVE_BGZF_THREADS=auto`; use `VCF_FAST_NATIVE_BGZF_THREADS=1` to force the single-thread gzip fallback behavior for comparisons or constrained environments.
- Added `make bench-v14` and `benchmark/run_v14_public_parallel_scale_benchmarks.sh` to compare single-thread BGZF, auto BGZF, auto BGZF plus predicate parallelism, explicit BGZF threads, and `bcftools`.
- Added a tracked v1.4 smoke report without new full-scale speed claims.

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
