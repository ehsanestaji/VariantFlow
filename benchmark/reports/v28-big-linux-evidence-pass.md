# VariantFlow v2.8 Big Linux Evidence Pass

This scaffold is the v3.0 release-gate evidence rollup. It should be filled only after Linux repeated runs with RSS and CPU metrics complete for the major performance families.

## Components

| component | target | status | required correctness gate | release relevance |
| --- | --- | --- | --- | --- |
| v2.3 BGZF pipeline | `bench-v23-pipeline` | existing report current from prior v2.3 pass | native modes match byte-for-byte; `bcftools` core records match | compressed public/stress filtering |
| v2.4 .vfi pushdown | `bench-v24-index` | full v3.0 public matrix completed once; redundant v2.8 rerun stopped after reproducing early rows | indexed output matches default streaming; public `FILTER == "PASS"` and `AF > 0.99` chunk-target 256 rows matched default and `bcftools` | high-skip selective filtering |
| v2.5 packed genotype | `bench-v25-genotype` | 100k and 1M accepted after large-LD parity normalizer fix | VCFtools parity | population-genetics memory and speed |
| v2.6 columnar workflow | `bench-v26-columnar` | completed v3.0 matrix | DuckDB results match normalized VCF or `bcftools` baselines | repeated analytical queries |

## v3.0 Evidence Status

- v2.4 `.vfi`: `VCF_FAST_INDEX_CHUNK_RECORDS` was added and checksum normalization was fixed for large public metadata. The full Docker/Linux public matrix completed. Public IGSR `FILTER == "PASS"` matched default native byte-for-byte and matched `bcftools` core records, skipping all chunks at 100k and 1M and measuring `122.95x` to `272.73x` faster than `bcftools`. Public IGSR `AF > 0.99` with chunk target `256` also matched default and `bcftools`, skipped `89.8%` of chunks at 100k and `86.9%` at 1M, and measured `4.93x` and `4.70x` faster than `bcftools`. Public `QUAL > 1000` and coarser AF chunk targets fell back safely or did not beat `bcftools`, so no broad `.vfi` claim is promoted.
- v2.5 packed genotype: the Docker/Linux 1M run staged `public-cohort.biallelic.1000000.vcf.gz` and completed frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, and Weir-Cockerham Fst outputs. The original detached container exited after the old parity checker was killed while materializing the 15M-row LD output. `benchmark/check_vcftools_parity.py` now uses disk-backed sorted LD group comparison, and the existing 925730-record 1M outputs passed normalized VCFtools parity. Accepted 1M rows are tracked in `benchmark/reports/v25-packed-genotype-benchmark.md`; HWE is correctly marked as slightly slower (`0.98x`) rather than a speed win.
- v2.6 columnar: the full Docker/Linux row-group/query matrix completed for row groups `8192`, `65536`, and `262144`; queries `qual_gt_30`, `dp_gt_40`, `filter_pass`, and `group_by_chrom_filter`; tiers `100000` and `1000000`; repeated queries `5`; runs `3`; warmup `1`. All rows matched normalized baselines.
- v2.8 full orchestrated pass: attempted in Docker as `v28-full-20260510-010807`. The run passed the initial verification block and entered a redundant full `bench-v24-index` rerun. It was intentionally stopped after more than 10 hours because the completed v2.4/v2.5/v2.6 reports already contained the correctness-matched evidence used here, and the remaining rerun was spending time on known low-skip/fallback rows. No additional speed claim is promoted from the stopped v2.8 container.

## Release gate

- `make verify`
- `cargo test --features htslib-static`
- `cargo clippy --features htslib-static --all-targets -- -D warnings`
- `make vcftools-parity`
- claim matrix contains no unsupported broad claims
- public reports include Linux RSS, CPU seconds, CPU-hour estimates, exact commands, tool versions, and caveats

## Claim Discipline

no broad best-tool claim is supported by this scaffold. VariantFlow claims should remain workflow-specific and report-backed.
