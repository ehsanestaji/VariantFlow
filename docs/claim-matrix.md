# VariantFlow Claim Matrix

Every claim below points to a tracked report row or remains explicitly
unproven. Smoke tests validate harness behavior only; they are not speed
claims.

| Workflow | Current status | Evidence | Competitor | Caveat |
|---|---|---|---|---|
| Native selective QUAL filtering on bounded IGSR BGZF | beats | `benchmark/reports/v14-public-parallel-scale-benchmark.md` reports core-record correctness and 13.44x to 13.47x over `bcftools filter` | bcftools | bounded chr22 region; requested 1M tier reached 191526 records |
| Native FORMAT aggregate stress filtering | beats | `benchmark/reports/v14-public-parallel-scale-benchmark.md` reports byte-for-byte native parity and 4.33x to 5.27x over `bcftools filter` | bcftools | deterministic stress data; public 453-sample FORMAT cohort evidence is tracked separately |
| Public FORMAT-heavy aggregate filtering | beats on measured 453-sample cohort tiers through full chromosome | `benchmark/reports/v17-public-format-baselines.md` reports matched core records on an ENA Ovis aries 453-sample FORMAT-rich cohort; 10k/50k/100k/250k/1M/full-chromosome tiers were 1.76x to 3.50x faster than `bcftools filter` | bcftools | Docker/Linux timing; 1M/full tiers use heavy-output mode with core-record correctness and `/dev/null` timed output |
| Native TSV selected-column conversion | beats | `benchmark/reports/v08-core-efficiency-benchmark.md` reports normalized TSV correctness and 2.54x over `bcftools query` | bcftools query | selected columns only |
| Simple native stats counts | beats | `benchmark/reports/v08-core-efficiency-benchmark.md` reports supported-count parity and 2.50x over `bcftools stats` | bcftools stats | rich bcftools stats parity is not claimed |
| Parquet export plus repeated DuckDB queries | beats | `benchmark/reports/v12-public-parallel-workflow-benchmark.md` reports normalized query matches and 3.18x to 25.67x amortized speedup | repeated bcftools scans | selected-column Parquet; Polars and PyArrow pending |
| BCF input and indexed region compatibility | matches | `benchmark/reports/compatibility-benchmark.md` reports correctness/indexability checks | HTSlib/bcftools | htslib-backed paths are not byte-preserving native output |
| BCF TSV compatibility path | complements | `benchmark/reports/compatibility-benchmark.md` records correctness with slower rows | bcftools query | optimization gap, not a speed claim |
| GATK SelectVariants / VariantFiltration workflows | not yet proven | no tracked report row yet | GATK | optional v1.7+ baseline |
| VCFtools filtering/stats workflows | not yet proven | no tracked report row yet | VCFtools | optional legacy baseline |
