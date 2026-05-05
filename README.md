# VCF-Fast

VCF-Fast is a selective execution engine for post-calling variant operations. It treats VCF/BCF as exchange formats, avoids parsing unused fields, preserves original records where possible, and tracks correctness/performance against trusted tools such as bcftools.

The current boost strategy is **Evidence First**: prove correctness and speed on reproducible synthetic and public datasets before claiming broader superiority or moving deeper into parallel, vectorized, or columnar internals.

## Why It Can Be Faster

- Selective parsing avoids unused INFO/FORMAT work.
- Original-record preservation avoids reconstruction cost for passing VCF records.
- Typed predicates avoid ad hoc string evaluation.
- The benchmark harness checks outputs against `bcftools`.
- Future gains should come from larger public evidence, stress datasets, parallel/vectorized execution, and Arrow/Parquet export.

## Language And Engine Direction

VCF-Fast stays Rust-first. Rust gives the project C-like performance, strict memory control, and safer concurrency without making memory safety a permanent tax on development speed. Optional C/htslib interop is used only for targeted compatibility work: BCF input, BGZF output, tabix-indexed region reads, and cases where htslib is clearly the fastest correct path.

## Current Evidence

| scenario | competitor | correctness check | current result | caveat |
|---|---|---|---|---|
| Synthetic 1M filters | `bcftools filter` | matched filtered core records | `1.62x` to `1.82x` faster | three-run container benchmark |
| Synthetic 1M TSV conversion | `bcftools query` | matched normalized TSV rows | `1.57x` faster | selected columns only |
| GIAB HG002 public-whole QUAL filters | `bcftools filter` | matched filtered core records | `1.80x` to `2.38x` faster on plain tiers; `1.89x` faster on 1M gzip | 100k gzip was `0.94x`, so gzip wins are tier-dependent |
| GIAB HG002 public-whole TSV conversion | `bcftools query` | matched normalized TSV rows | `1.13x` faster at 1M | 10k/100k were `0.80x` and `0.96x` |
| IGSR chr22 public-whole QUAL filters | `bcftools filter` | matched filtered core records | `4.85x` to `5.71x` faster on measured 10k/100k tiers | 1M deferred after >13 GB generated intermediate |
| IGSR chr22 public-whole TSV conversion | `bcftools query` | matched normalized TSV rows | `1.22x` faster at 10k; `0.87x` at 100k | TSV path is mixed |
| IGSR chr22 public-heavy gzip filter/TSV | `bcftools filter` / `bcftools query` | matched filtered core records / normalized TSV rows | `5.23x` to `5.65x` faster for QUAL filtering at 100k/1M; `1.08x` to `1.10x` faster for TSV at 100k/1M | bounded chr22 region |
| IGSR chr22 public-heavy after v0.8 byte-core surgery | `bcftools filter` / `bcftools query` | supported correctness matched: filtered core records / normalized TSV rows | Heavy QUAL gzip input `6.01x` faster; Heavy Convert TSV gzip input `1.13x` faster | bounded chr22:1-20000000 region; repeated local run with 3 measured runs and 1 warmup |
| IGSR chr22 threaded native BGZF input | `bcftools filter` | default and threaded VCF-Fast matched `bcftools` filtered core records | threaded native BGZF input was `1.85x` to `2.00x` faster than default native gzip/BGZF input and `9.90x` to `11.87x` faster than `bcftools filter` on 10k/100k/1M | opt-in `VCF_FAST_NATIVE_BGZF_THREADS=4`; bounded chr22 region; ordinary gzip remains single-thread fallback |
| IGSR chr22 indexed-region QUAL filters | `bcftools view -r` + `bcftools filter` | matched filtered core records | `1.47x` faster at 10k and 100k | htslib-backed path, not line-preserving native output |
| IGSR chr22 indexed-region TSV/stats | `bcftools query` / `bcftools stats` | matched TSV rows / overlapping counts | `0.71x` to `0.72x` | bcftools faster; compatibility path needs optimization |
| Stress 1M filters with unused INFO/FORMAT/sample payload | `bcftools filter` | matched filtered core records | `1.96x` to `2.45x` faster on plain VCF | synthetic stress shape |
| Stress 1M selected-sample FORMAT filters | `bcftools filter` | matched filtered core records | `1.99x` to `2.06x` faster | single selected sample, synthetic stress shape |
| Stress 1M TSV conversion | `bcftools query` | matched normalized TSV rows | `1.20x` faster | selected columns only |
| Stress 1M stats | `bcftools stats` | matched overlapping record count | `1.53x` faster | richer stats equivalence pending |
| Stress 1M after v0.8 byte-core surgery | `bcftools filter` / `bcftools query` / `bcftools stats` | supported correctness matched: filtered core records, normalized TSV rows, and stats records | filters `3.14x` to `6.24x` faster; TSV `2.54x` faster; stats `2.50x` faster | synthetic stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD; repeated local run with 3 measured runs and 1 warmup |
| v0.9 native expression parity stress evidence | `bcftools filter` | matched filtered core records for arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and `ANY`/`ALL` sample aggregate predicates | `2.41x` to `5.18x` faster at 100k deterministic stress records; `2.66x` to `3.90x` faster at 10k | synthetic stress expression evidence only; public v0.9 expression rows pending |
| Compatibility proof | `bcftools`, `tabix`, HTSlib | BCF/region/BGZF correctness and indexability | correctness matched; v0.7 near parity or faster for BCF filter, indexed-region filter/stats, and near parity for BGZF output | BCF TSV still trails `bcftools query` |
| v1.0 native Parquet export | `bcftools query` TSV projection | Parquet schema/null semantics verified by Arrow readback tests; TSV and bcftools row counts matched input records | Parquet export was `1.93x` to `1.94x` faster than `bcftools query` on 10k/100k deterministic stress projection | native TSV remains faster than Parquet; synthetic stress only; downstream DuckDB/Polars evidence pending |
| v1.0 columnar workflow | repeated `bcftools view` scans | DuckDB row counts over VCF-Fast Parquet matched repeated `bcftools view -H` row counts | export-plus-five DuckDB queries measured `23.96x` to `48.45x` faster than five repeated `bcftools view` scans on bounded IGSR chr22 10k/100k tiers | row-count workflow only; broader query shapes and DuckDB/Polars/PyArrow baselines pending |

Detailed evidence lives in:

- `benchmark/reports/synthetic-filter-benchmark.md`
- `benchmark/reports/public-dataset-benchmark.md`
- `benchmark/reports/stress-speed-benchmark.md`
- `benchmark/reports/format-filter-benchmark.md`
- `benchmark/reports/compatibility-benchmark.md`
- `benchmark/reports/public-whole-cohort-benchmark.md`
- `benchmark/reports/v07-heavy-run-benchmark.md`
- `benchmark/reports/v08-core-efficiency-benchmark.md`
- `benchmark/reports/v09-expression-parity-benchmark.md`
- `benchmark/reports/v10-compressed-input-benchmark.md`
- `benchmark/reports/v10-parquet-export-benchmark.md`
- `benchmark/reports/v10-columnar-workflow-benchmark.md`
- `docs/contribution-map.md`

Public evidence now supports the native selective-filter claim on measured GIAB and IGSR tiers. The v0.7 heavy run also shows the optimized native TSV path can beat `bcftools query` on bounded sample-rich gzip workloads through 1M records, v0.9 stress evidence shows the expanded native expression engine beating `bcftools filter` on measured deterministic stress cases, and the v1.0 slices show opt-in threaded native BGZF input, typed Parquet export, and a measured bounded-IGSR row-count workflow win through DuckDB. Honest gaps remain: BCF TSV still trails `bcftools query`, public v0.9 expression rows are pending, ordinary gzip is not parallelized, and richer Parquet workflow query shapes are still pending.

## Milestones

1. `v0.1 Evidence Baseline`: streaming filter, stats/diff, TSV conversion, synthetic and GIAB benchmark reports.
2. `v0.2 Public Benchmark Expansion`: IGSR chr22 public-region, repeated hyperfine runs, 1M-record synthetic cases, memory/throughput reporting.
3. `v0.3 Stress And Speed`: synthetic stress VCFs with many unused INFO/FORMAT/sample fields, parser hot-path improvements, and stress benchmark reporting.
4. `v0.4 FORMAT-Aware Filtering`: support `FORMAT/GT`, `FORMAT/DP`, `FORMAT/GQ`, selected sample predicates, bcftools comparison.
5. `v0.5 Compatibility Proof`: optional htslib-backed BCF input, BGZF output, and tabix-indexed region reads while preserving the Rust-native selective streaming path.
6. `v0.6 Public Whole-Cohort Evidence`: tiered local GIAB/IGSR runs, repeated benchmark reports, memory trends, compatibility benchmarks, and exact claim matrix updates.
7. `v0.7 Heavy-Run And Htslib Optimization`: avoid giant public-data intermediates, tune htslib compatibility paths, and report path-specific bottlenecks before broader claims.
8. `v0.8 Core Efficiency And Evidence`: byte-slice native record views, cached INFO scanning, byte-backed expression evaluation, native filter/stats hot-path migration, and repeated post-surgery evidence.
9. `v0.9 Expression Parity`: arbitrary selected `INFO/*` and `FORMAT/*`, selected sample predicates, sample `ANY`/`ALL`, and documented compatibility with common `bcftools filter` semantics.
10. `v1.0 Parallel And Columnar`: opt-in native threaded BGZF input, native selected-column Parquet export, DuckDB columnar workflow evidence, broader parallel BGZF execution, release-grade claim matrix, installer docs, and reproducible benchmark reports.

## Quickstart

```bash
cargo build
cargo test
make verify

vcf-fast filter input.vcf.gz --where "QUAL > 30" -o output.vcf.gz
vcf-fast stats input.vcf.gz
vcf-fast diff a.vcf.gz b.vcf.gz -o diff.tsv
vcf-fast convert input.vcf.gz --to tsv -o variants.tsv
vcf-fast convert input.vcf.gz --to parquet -o variants.parquet

cargo build --features htslib-static
vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where "QUAL > 30" -o output.vcf
vcf-fast filter input.bcf --where "QUAL > 30" -o output.vcf
vcf-fast filter input.vcf --where "QUAL > 30" --compression bgzf -o output.vcf.gz
vcf-fast convert input.bcf --region chr22:1-20000000 --to tsv -o variants.tsv
vcf-fast stats input.bcf --region chr22:1-20000000

cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf

docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast cargo test
docker run --rm -v "$PWD:/work" -e VCF_FAST_BENCH_SIZES="10000 100000" vcf-fast make bench-smoke

benchmark/download_public_data.sh all
make bench-public
make bench-public-region
make bench-compat
make bench-v09
make bench-v10-compressed
make bench-v10-parquet
make bench-v10-columnar
make bench-v06-smoke
```

## Current CLI

```bash
vcf-fast filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
vcf-fast filter tests/data/example.vcf --where "QUAL >= 30 && DP > 10" -o tests/output/dp.vcf
vcf-fast filter tests/data/example.vcf --where "(QUAL > 55 || INFO/DP > 45) && FILTER == \"PASS\"" -o tests/output/grouped.vcf
vcf-fast filter tests/data/example.vcf.gz --where "AF > 0.01 && FILTER == \"PASS\"" -o tests/output/af.vcf.gz
vcf-fast filter tests/data/expression_parity.vcf --where "INFO/MQ >= 50 && INFO/CSQ == \"synonymous_variant\"" -o tests/output/info_expr.vcf
vcf-fast filter tests/data/expression_parity.vcf --sample HG002 --where "FORMAT/AD > 8 && FORMAT/FT == \"PASS\"" -o tests/output/format_expr.vcf
vcf-fast filter tests/data/expression_parity.vcf --where "ANY(FORMAT/DP > 20)" -o tests/output/any_dp.vcf
vcf-fast filter tests/data/expression_parity.vcf --where "ALL(FORMAT/GQ >= 30)" -o tests/output/all_gq.vcf
VCF_FAST_NATIVE_BGZF_THREADS=4 vcf-fast filter input.vcf.gz --where "QUAL > 30" -o output.vcf
cargo run --features htslib-static -- filter tests/data/compat_example.vcf --where "QUAL > 30" --compression bgzf -o tests/output/compat.vcf.gz
vcf-fast stats tests/data/example.vcf
vcf-fast diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o tests/output/diff.tsv
vcf-fast convert tests/data/example.vcf --to tsv -o tests/output/variants.tsv
vcf-fast convert tests/data/example.vcf --to parquet -o tests/output/variants.parquet
```

## Native Filter Support

- Inputs: `.vcf`, `.vcf.gz`
- Outputs: `.vcf`, `.vcf.gz`
- Optional native BGZF input acceleration: set `VCF_FAST_NATIVE_BGZF_THREADS=<positive integer>` for BGZF `.vcf.gz` inputs. Ordinary gzip input falls back to the single-thread flate2 path.
- Site fields: `QUAL`, `CHROM`, `POS`, `FILTER`
- INFO fields: arbitrary `INFO/<KEY>` predicates. `DP` and `AF` remain aliases for `INFO/DP` and `INFO/AF`.
- FORMAT fields: arbitrary selected-sample `FORMAT/<KEY>` predicates require `--sample <name>`.
- Sample aggregates: `ANY(FORMAT/<KEY> op literal)` and `ALL(FORMAT/<KEY> op literal)` scan all sample columns.
- Operators: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Boolean operators: `&&`, `||`
- Grouping: parentheses

Numeric `INFO/<KEY>` and `FORMAT/<KEY>` comparisons pass when any comma-separated numeric value satisfies the predicate. String comparisons use byte-exact quoted literals against the full raw field value. Missing INFO keys, missing FORMAT keys, empty values, flag-only INFO entries, and `.` values make the predicate false. `ALL(FORMAT/<KEY> op literal)` therefore requires every sample to have a present satisfying value, while `ANY(FORMAT/<KEY> op literal)` requires at least one present satisfying value. `ANY` and `ALL` require a `#CHROM` header with at least one sample column.

## Limitations

The default build is a line-preserving streaming filter, not the future full columnar execution engine. Native gzip output is valid gzip-compressed VCF text but is not promised to be tabix-indexable. Opt-in native threaded BGZF input accelerates BGZF `.vcf.gz` reads only; ordinary single-stream gzip is still decoded by the existing flate2 fallback. With `--features htslib` or `--features htslib-static`, `--compression bgzf`, `.bcf` input, and `--region` use htslib compatibility paths. Those paths guarantee valid VCF output and bcftools-equivalent core records for supported predicates, but they do not preserve original record text byte-for-byte. The htslib-backed paths keep the older compatibility surface and reject native-only aggregate predicates with `ANY/ALL FORMAT predicates are not implemented for htslib-backed input in v0.9`. Parquet export is native `.vcf`/`.vcf.gz` selected-column export only in this slice; BCF/region Parquet, richer DuckDB/Polars/PyArrow workflow queries, broad whole-cohort expression benchmarks, and public v0.9 runtime win claims are still pending.

## Stats Output

`stats` writes JSON to stdout with site-level and allele-level metrics:

- record count in `variants`
- allele-level `snps` and `indels`
- `variants_per_chromosome`
- `qual` and `af` count/min/max/mean summaries
- `missing_filter_values`
- `transition_transversion_ratio`

## Diff Output

`diff` compares variant keys as `CHROM + POS + REF + ALT`, writes a TSV to `-o`, and prints summary counts to stderr:

```text
status	chrom	pos	ref	alt
only_in_a	1	100	A	G
shared	1	200	C	T
only_in_b	2	400	G	A
```

## Convert Output

`convert --to tsv` writes analysis-friendly TSV with stable columns:

```text
CHROM POS ID REF ALT QUAL FILTER INFO/DP INFO/AF
```

Missing values are written as `.`. Multi-allelic `INFO/AF` values are preserved as the original comma-separated value.

`convert --to parquet` writes the same selected projection for native `.vcf` and `.vcf.gz` inputs. The first Parquet schema is typed for analysis: `POS` is int64, `QUAL` is nullable float64, `INFO/DP` is nullable int64, and `INFO/AF` remains nullable UTF-8 so comma-separated AF values stay lossless. Missing numeric values become nulls.

## Development

```bash
make fmt
make clippy
make test
make build
make verify
cargo test --features htslib-static
```

Run the smoke command:

```bash
cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
```

Run the synthetic benchmark and correctness smoke harness:

```bash
make bench-smoke
```

Set `VCF_FAST_BENCH_SIZES` to control synthetic dataset sizes. If `hyperfine` and `bcftools` are installed, the harness times VCF-Fast against the comparable bcftools filter/query command and compares filtered core records or normalized TSV rows. The TSV baseline uses `bcftools query -u` so public VCFs without optional `INFO/DP` or `INFO/AF` header definitions still produce `.` values for those columns.

The current tracked benchmark report is in `benchmark/reports/synthetic-filter-benchmark.md`.

Run the synthetic stress benchmark with unused INFO/FORMAT/sample payloads:

```bash
make bench-stress
VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="100000 1000000" make bench-smoke
```

Download public benchmark data into the ignored local cache:

```bash
benchmark/download_public_data.sh giab-hg002
benchmark/download_public_data.sh igsr-chr22
```

Run public-data benchmark modes after downloading:

```bash
VCF_FAST_BENCH_MODE=public-small VCF_FAST_BENCH_SIZES="10000" make bench-smoke
VCF_FAST_BENCH_MODE=public-region VCF_FAST_BENCH_SIZES="10000" make bench-smoke
```

Run compatibility proof checks:

```bash
cargo test --features htslib-static
```

The optional htslib backend is selected automatically for `.bcf` input, `--region`, or `--compression bgzf`. Default builds return a clear error for those htslib-only operations.

## Docker

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast cargo test
```
