# VCF-Fast Contribution Map

VCF-Fast is not intended to be only a faster VCF scanner. The project is building toward a benchmarked execution engine for post-calling variant operations.

## Current Implemented Contributions

### Selective Parsing

The filter engine parses only fields required by the expression. For example, `QUAL > 30` does not parse INFO values, while `INFO/DP > 40` extracts the INFO field needed for that predicate.

Evidence:

- Filter implementation uses required-field analysis from the expression AST.
- Tests cover borrowed record parsing that skips unneeded INFO parsing.
- Shared borrowed VCF field parsing avoids per-record column vectors in filter, stats, and TSV conversion paths.
- Shared byte-based INFO scanning avoids repeated allocation-heavy `split` chains for filter predicates, TSV conversion, and stats.
- v0.8 moved native filter and stats onto shared byte-slice `RecordView`/`InfoView` parsing and byte-backed expression evaluation, reducing line-level string parsing and repeated INFO scans.
- v1.0 first-slice compressed input added threaded native BGZF reading through `VCF_FAST_NATIVE_BGZF_THREADS`, while ordinary gzip remains on the existing flate2 fallback. v1.4 turns the measured public BGZF lesson into default behavior: an unset `VCF_FAST_NATIVE_BGZF_THREADS` uses auto-capped native BGZF workers, `auto` requests that policy explicitly, and `1` forces the single-thread fallback behavior.
- v1.1 adds opt-in native predicate parallelism through `VCF_FAST_NATIVE_FILTER_THREADS`, evaluating bounded batches in parallel and writing passing original lines back in input order.
- Benchmarks show speedups for QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases in the synthetic benchmark harness.
- Stress benchmarks show speedups when records contain many unused INFO/FORMAT/sample fields.
- Selected-sample FORMAT filtering reads only the requested sample column for arbitrary native `FORMAT/<KEY>` predicates.
- v0.9 native expression parity adds arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>` with `--sample`, and `ANY(FORMAT/<KEY>)` / `ALL(FORMAT/<KEY>)` sample aggregate predicates, with deterministic stress evidence tracked against `bcftools filter`.

### Original-Record Preservation

The filter command writes original VCF header and record lines when records pass. This avoids expensive record reconstruction and preserves unsupported fields, sample columns, and formatting.

Evidence:

- CLI tests assert exact passing record preservation.
- gzip input/output tests verify compressed streaming behavior.

### Typed Expression Evaluation

The expression engine parses filter strings into an AST with typed numeric and string comparisons, boolean precedence, and parenthesized grouping.

Supported examples:

```text
QUAL > 30
INFO/DP > 10 && INFO/AF > 0.01
(QUAL > 55 || INFO/DP > 45) && FILTER == "PASS"
FORMAT/DP > 20
FORMAT/GQ >= 30
FORMAT/GT == "0/1"
INFO/MQ >= 50
INFO/CSQ == "synonymous_variant"
FORMAT/AD > 8
ANY(FORMAT/DP > 20)
ALL(FORMAT/GQ >= 30)
```

Selected-sample FORMAT predicates require `--sample <name>`. `ANY(FORMAT/<KEY> op literal)` and `ALL(FORMAT/<KEY> op literal)` scan all sample columns in native `.vcf` and `.vcf.gz` filtering. Missing INFO keys, missing FORMAT keys, empty values, flag-only INFO entries, and `.` values make the predicate false. Numeric comparisons use any comma-separated numeric value; string comparisons are byte-exact against quoted literals.

### Canonical Variant Keys

The `diff` command compares variants by `CHROM + POS + REF + ALT`, producing shared and unique variant sets as TSV.

Evidence:

- Integration tests verify shared, only-in-A, and only-in-B outputs.

### Reproducible Benchmarking

The benchmark harness builds deterministic synthetic VCFs, compares VCF-Fast against bcftools, checks output equivalence on filtered core records, and records results in Markdown.

Evidence:

- Docker installs `bcftools`, `hyperfine`, and the Rust toolchain.
- Current benchmark report: `benchmark/reports/synthetic-filter-benchmark.md`.

### TSV Conversion

The `convert --to tsv` command exports VCF records into stable, analysis-friendly columns while preserving missing values and comma-separated INFO/AF values. The native `convert --to parquet` command exports the same selected projection with typed `POS`, nullable `QUAL`, nullable `INFO/DP`, and lossless string `INFO/AF`. The first columnar workflow harness measures export-once, repeated-query behavior through DuckDB.

Evidence:

- Integration tests cover plain and gzip input.
- Parquet integration tests read produced files through Arrow and verify schema, row count, nulls, typed numeric columns, and multi-value AF preservation.
- Benchmark harness compares normalized TSV rows against `bcftools query`.
- The v1.0 columnar workflow harness compares DuckDB queries over VCF-Fast Parquet output with repeated `bcftools` scans over the original VCF/BGZF input.

### Compatibility Interop

The default engine remains Rust-native and dependency-light. With `--features htslib` or `--features htslib-static`, VCF-Fast can route compatibility-only operations through HTSlib: BCF input, tabix-indexed region reads, and BGZF output.

Evidence:

- Backend selector tests cover native vs htslib routing.
- Default-build CLI tests verify clear errors for `.bcf`, `--region`, and `--compression bgzf`.
- htslib-enabled integration tests cover BCF filtering, BCF TSV conversion, indexed region filtering, indexed BCF region stats, and BGZF output that is gzip-readable and tabix-indexable.
- Competitor design notes are tracked in `docs/competitor-notes.md`.

### Release Hardening

VCF-Fast now has a versioned CLI, release/install documentation, a changelog, generated benchmark table, and GitHub release workflow scaffolding.

Evidence:

- `variantflow --version` and the `vcf-fast` compatibility alias are backed by the Cargo package version.
- `docs/release.md` documents source, Docker, htslib feature, public-data, and benchmark reproduction paths.
- `docs/public-benchmark-table.md` is generated from tracked benchmark reports by `benchmark/generate_public_benchmark_table.py`.
- `.github/workflows/release.yml` builds macOS/Linux release archives for version tags.

## Claims Supported So Far

- Correctness: VCF-Fast matches bcftools filtered core records for supported synthetic QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases.
- Performance: On the tracked 1M synthetic benchmark run, VCF-Fast was `1.62x` to `1.82x` faster than bcftools across measured supported filter cases and `1.57x` faster for TSV conversion.
- Public whole-cohort tiers: On the v0.6 Docker run, GIAB HG002 matched bcftools outputs and measured `1.80x` to `2.38x` faster for plain QUAL filtering, `1.89x` faster for 1M gzip QUAL filtering, and `1.13x` faster for 1M TSV conversion; smaller GIAB TSV/gzip tiers were mixed.
- Public IGSR tiers: On the v0.6 Docker run, IGSR chr22 10k/100k tiers matched bcftools outputs and measured `4.85x` to `5.71x` faster for QUAL filtering; TSV measured `1.22x` at 10k and `0.87x` at 100k. IGSR 1M whole-file is deferred because the generated plain VCF exceeded 13 GB during the balanced run.
- Public indexed region: On the v0.6 Docker run, IGSR chr22 indexed-region filtering matched bcftools and measured `1.47x` faster at 10k and 100k; region TSV and stats matched correctness but measured `0.71x` to `0.72x`, so bcftools is faster there.
- Public heavy sample-rich path: On the v0.7 bounded public-heavy runs after optimization, VCF-Fast matched bcftools outputs and measured `5.23x` to `5.65x` faster for gzip QUAL filtering at 100k/1M and `1.08x` to `1.10x` faster for gzip TSV conversion at 100k/1M. The bottleneck was narrowed to wide-line materialization plus gzip backend cost; the fix uses streaming TSV field reads, tail skipping after INFO, and flate2 with `zlib-ng`.
- Stress speed: On the tracked 1M synthetic stress benchmark with 40 unused INFO fields and 16 samples, VCF-Fast matched bcftools outputs and measured `1.96x` to `2.45x` faster for plain filter cases, `1.20x` faster for TSV conversion, and `1.53x` faster for overlapping stats record counts.
- FORMAT-aware filtering: On the tracked 1M synthetic stress benchmark, selected-sample `FORMAT/DP`, `FORMAT/GQ`, and `FORMAT/GT` filters matched bcftools filtered core records and measured `1.99x` to `2.06x` faster.
- v0.9/v1.8 native expression parity: Fixture-backed tests and the v0.9 benchmark report cover arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>` with `--sample`, and native sample aggregate predicates such as `ANY(FORMAT/DP > 20)` and `ALL(FORMAT/GQ >= 30)`. On deterministic stress VCFs with 40 unused INFO fields and 16 samples, VCF-Fast matched `bcftools filter` core records and measured `2.66x` to `3.90x` faster at 10k records and `2.41x` to `5.18x` faster at 100k records. v1.8 extends that to Docker/Linux public ENA 453-sample cohort rows for DP, GQ, AD, selected-sample DP, and mixed QUAL+FORMAT expressions, with matched core records and `3.22x` to `8.77x` speedups.
- v1.0 threaded native BGZF input: On bounded IGSR chr22 BGZF input, VCF-Fast matched `bcftools filter` core records. With `VCF_FAST_NATIVE_BGZF_THREADS=4`, it measured `1.85x` to `2.00x` faster than the default native gzip/BGZF path and `9.90x` to `11.87x` faster than `bcftools filter` at 10k/100k/1M records. This is BGZF-only evidence; ordinary gzip remains a single-thread fallback.
- v1.0 Parquet export: Native `convert --to parquet` writes a typed selected-column Parquet file for `.vcf` and `.vcf.gz` inputs. Correctness is covered by Arrow readback tests. On deterministic stress data, Parquet export measured `1.93x` to `1.94x` faster than the comparable `bcftools query` TSV projection, while native TSV remained faster than native Parquet.
- v1.0 columnar workflow: On bounded IGSR chr22 public-heavy BGZF data, VCF-Fast Parquet export plus five DuckDB row-count queries matched repeated `bcftools view -H` row counts and measured `23.96x` faster at 10k records and `48.45x` faster at 100k records than five repeated `bcftools view` scans. This supports a narrow export-once, query-many claim for the measured row-count workflow.
- v1.1 parallel native filter: On deterministic stress data with `ANY(FORMAT/AD > 80)`, `VCF_FAST_NATIVE_FILTER_THREADS=4` plus ordered batches matched default native output byte-for-byte and matched `bcftools filter` core records. It measured `1.26x` to `1.96x` faster than default native and `2.90x` to `4.89x` faster than `bcftools filter` at 10k/100k records.
- v1.2 public parallel and workflow evidence: On bounded IGSR chr22 public-heavy BGZF data, default, parallel, threaded BGZF, and combined native filter outputs matched byte-for-byte and matched `bcftools filter` core records. Threaded BGZF input was fastest on this I/O-bound QUAL filter: `2.44x` to `2.55x` faster than default native and `15.13x` to `15.63x` faster than `bcftools`; combined threaded BGZF plus parallel native also beat default, while predicate-only parallel was slower at `0.88x` to `0.89x`. On deterministic stress `ANY(FORMAT/AD > 80)` data, parallel native measured `1.91x` to `1.98x` faster than default native and `4.85x` to `5.10x` faster than `bcftools` through 1M records. VCF-Fast Parquet export plus five DuckDB queries for `QUAL`, `INFO/DP`, `FILTER`, and grouped `CHROM,FILTER` matched normalized `bcftools` baselines and measured `3.18x` to `25.67x` amortized speedups. The requested public 1M bounded-region tier contained 191526 available records.
- v1.4 public parallel scale: Native BGZF input now uses auto-capped reader workers by default, with `VCF_FAST_NATIVE_BGZF_THREADS=1` as the single-thread escape hatch. On the full v1.4 measured run, single-thread, auto BGZF, auto BGZF plus predicate parallelism, and explicit BGZF native outputs matched byte-for-byte, and auto core records matched `bcftools filter`. Default auto BGZF measured `2.26x` to `2.39x` faster than forced single-thread native and `13.44x` to `13.47x` faster than `bcftools` on 100k and bounded 191526-record public rows. On deterministic stress `ANY(FORMAT/AD > 80)`, opt-in predicate parallelism measured `1.77x` to `2.01x` faster than default native and `4.33x` to `5.27x` faster than `bcftools`.
- Compatibility proof: Optional htslib-backed paths cover BCF input, indexed region reads, and BGZF output. v0.7 typed TSV/stats optimization matched correctness and moved several compatibility paths to near parity or faster, including 1M BCF filter `1.05x`, indexed-region filter `1.25x`, and indexed BCF stats `1.05x`; BCF TSV still trails `bcftools query` at `0.50x` for 1M.
- v0.8 byte-core evidence: On the repeated post-surgery benchmark, VCF-Fast matched supported correctness checks. The measured stress and IGSR public-heavy results are recorded in `benchmark/reports/v08-core-efficiency-benchmark.md`: stress filters were `3.14x` to `6.24x` faster, stress TSV was `2.54x` faster, stress stats were `2.50x` faster, public-heavy QUAL was `6.01x` faster, and public-heavy TSV was `1.13x` faster; all v0.8 rows were measured wins, with caveats limited to synthetic stress shape and bounded chr22 region.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, and `benchmark/reports/public-whole-cohort-benchmark.md` | `bcftools filter` | `1.62x` to `1.82x` faster on supported 1M synthetic cases; GIAB public tiers up to `2.38x`; IGSR public tiers up to `5.71x`; indexed-region filtering `1.47x` | IGSR whole 1M deferred after >13 GB intermediate |
| Stress selective parsing | `benchmark/reports/stress-speed-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | `1.96x` to `2.45x` faster on 1M plain stress filters; `1.20x` TSV speedup; `1.53x` stats speedup | Synthetic stress shape, not a public cohort |
| Selected-sample FORMAT filtering | `benchmark/reports/format-filter-benchmark.md` | `bcftools filter` | `1.99x` to `2.06x` faster on 1M selected-sample FORMAT filters | Single selected sample only; synthetic stress shape |
| v0.9/v2.0 native expression parity | `tests/expr_tests.rs`, `tests/filter_cli_tests.rs`, `benchmark/reports/v09-expression-parity-benchmark.md`, `benchmark/reports/v18-public-format-expression-breadth.md`, `benchmark/reports/v19-second-public-format-cohort.md`, `benchmark/reports/v20-human-format-cohort.md` | `bcftools filter` | Arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and `ANY(FORMAT/<KEY>)` / `ALL(FORMAT/<KEY>)` semantics are documented and fixture-tested; deterministic stress rows measured `2.41x` to `5.18x` faster at 100k records, Docker/Linux public ENA 453-sample DP/GQ/AD/selected-sample/mixed FORMAT rows measured `3.22x` to `8.77x`, ENA Dutch Genebank Cattle 29-sample full-file rows measured `1.46x` to `26.66x`, and DDBJ CHM13 3715-sample human bounded 1k/10k/50k rows measured `4.74x` to `17.78x` faster with matched core records | Public FORMAT evidence now spans sheep, cattle, and human cohort shapes; the human evidence is bounded rather than full 27 GB; htslib-backed aggregate predicates remain unsupported |
| Threaded native BGZF input | `benchmark/reports/v10-compressed-input-benchmark.md` | `bcftools filter` | Opt-in `VCF_FAST_NATIVE_BGZF_THREADS=4` matched core records and measured `9.90x` to `11.87x` faster than `bcftools filter`; also `1.85x` to `2.00x` faster than the default native reader on the same BGZF inputs | Bounded IGSR chr22 10k/100k/1M only; ordinary gzip is not parallelized |
| Parallel native predicate evaluation | `tests/filter_cli_tests.rs`, `benchmark/reports/v11-parallel-native-filter-benchmark.md` | default native filter, `bcftools filter` | Opt-in `VCF_FAST_NATIVE_FILTER_THREADS=4` matched default output byte-for-byte and measured `1.26x` to `1.96x` faster than default native on CPU-heavy aggregate FORMAT stress filters | Synthetic stress only; I/O-bound filters may not benefit |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons, arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and native sample aggregates | htslib-backed aggregate predicates are rejected in v0.9 |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked; synthetic 1M `1.57x`; GIAB 1M `1.13x`; public-heavy 100k/1M gzip `1.08x` to `1.10x` after v0.7 optimization | Mixed public results: GIAB 10k/100k, IGSR 100k, and BCF TSV trail bcftools |
| Parquet export | `tests/convert_cli_tests.rs`, `benchmark/reports/v10-parquet-export-benchmark.md` | `bcftools query` for TSV projection baseline | Native `.vcf`/`.vcf.gz` selected-column Parquet export works with typed numeric/null semantics and measured `1.93x` to `1.94x` faster than `bcftools query` on stress projection | Native TSV is still faster than native Parquet; downstream workflow checks pending |
| Columnar workflow | `benchmark/reports/v10-columnar-workflow-benchmark.md` | repeated `bcftools view` scans | Export once plus five DuckDB row-count queries over VCF-Fast Parquet measured `23.96x` to `48.45x` faster than five repeated `bcftools view` scans on bounded IGSR chr22 10k/100k tiers | Row-count workflow only; richer predicate/projection workflows and Polars/PyArrow baselines pending |
| Public parallel/workflow expansion | `benchmark/reports/v12-public-parallel-workflow-benchmark.md` | default native, parallel native, threaded BGZF input, `bcftools filter`, normalized `bcftools` query/view baselines | v1.2 measured public-heavy BGZF filter rows, 1M stress parallel rows, and DuckDB `QUAL`, `INFO/DP`, `FILTER`, and grouped `CHROM,FILTER` workflows. Public threaded BGZF input was `2.44x` to `2.55x` faster than default native and `15.13x` to `15.63x` faster than `bcftools`; stress parallel native was `1.91x` to `1.98x` faster than default and `4.85x` to `5.10x` faster than `bcftools`; amortized DuckDB workflows were `3.18x` to `25.67x` faster than repeated `bcftools` scans | Public requested 1M tier reached 191526 available records in the bounded region; predicate-only parallel was slower on the I/O-bound public QUAL filter |
| v1.4 public parallel scale policy | `src/io.rs`, `benchmark/reports/v14-public-parallel-scale-benchmark.md` | `bcftools filter` | Default native `.vcf.gz` BGZF reads now use auto-capped native BGZF workers; measured public 100k and bounded 191526-record rows matched correctness and showed auto BGZF `2.26x` to `2.39x` faster than forced single-thread native and `13.44x` to `13.47x` faster than `bcftools`; stress FORMAT aggregate rows showed opt-in predicate parallelism `1.77x` to `2.01x` faster than default native and `4.33x` to `5.27x` faster than `bcftools` | Public tier is bounded chr22:1-20000000, requested 1M reached 191526 available records; RSS was `n/a` on this macOS run; public 453-sample FORMAT cohort evidence is tracked in v1.7 |
| v1.7 public FORMAT-rich aggregate evidence | `benchmark/reports/v17-public-format-baselines.md` | `bcftools filter` | ENA Ovis aries 453-sample FORMAT-rich cohort rows matched `bcftools` core records; Docker/Linux repeated 10k/50k/100k/250k/1M/full-chromosome `N_PASS(FORMAT/AD[1] > 10)` tiers measured `1.76x` to `3.50x` faster | 1M/full tiers use heavy-output mode: correctness streams compact core records and timed output goes to `/dev/null`; broader expression breadth is tracked in v1.8 |
| v1.8 public FORMAT expression breadth | `benchmark/reports/v18-public-format-expression-breadth.md` | `bcftools filter` | Docker/Linux ENA Ovis aries 453-sample 1M/full rows for `ANY(FORMAT/DP > 20)`, `ALL(FORMAT/GQ >= 30)`, `N_PASS(FORMAT/AD[1] > 10) >= 10`, selected-sample `FORMAT/DP > 20`, and `QUAL > 30 && ANY(FORMAT/DP > 20)` matched `bcftools` core records and measured `3.22x` to `8.77x` faster | Hyperfine reported outliers on some rows; evidence is still one public FORMAT-rich cohort |
| v1.9 second public FORMAT-rich cohort | `benchmark/reports/v19-second-public-format-cohort.md` | `bcftools filter` | ENA Dutch Genebank Cattle `ERZ18456468` 29-sample full Y-chromosome rows for `ANY(FORMAT/DP > 20)`, `ALL(FORMAT/GQ >= 30)`, `N_PASS(FORMAT/AD[1] > 10) >= 2`, selected-sample `FORMAT/DP > 20`, and `QUAL > 30 && ANY(FORMAT/DP > 20)` matched `bcftools` core records and measured `1.46x` to `26.66x` faster | This removes dependence on one sheep cohort, but the second public cohort is cattle rather than human/plant; Mayo human 629-sample VCF-Miner downloads returned 403 during automated validation |
| v2.0 human FORMAT-rich cohort | `benchmark/reports/v20-human-format-cohort.md` | `bcftools filter` | DDBJ CHM13 public-human-genomes 3715-sample chr22 bounded 1k/10k/50k rows for `ANY(FORMAT/DP > 20)`, `ALL(FORMAT/GQ >= 30)`, `N_PASS(FORMAT/AD[1] > 10) >= 10`, selected-sample `FORMAT/DP > 20`, and `QUAL > 30 && ANY(FORMAT/DP > 20)` matched `bcftools` core records and measured `4.74x` to `17.78x` faster | Bounded streaming tiers only; full 27 GB remote VCF is not cached by default |
| Public data benchmarking | `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/v07-heavy-run-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | GIAB HG002, IGSR chr22, stress, and bounded public-heavy 100k/1M runs measured with correctness, runtime, throughput, and RSS reporting | Whole-cohort sample-rich public evidence beyond bounded chr22 region still pending |
| Compatibility interop | `tests/compatibility_cli_tests.rs`, `tests/compatibility_unit_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | `bcftools`, `tabix`, HTSlib | BCF input, indexed region reads, and BGZF output are feature-gated, tested, benchmarked, and near parity/faster on several v0.7 rows | BCF TSV remains slower than bcftools query |
| Release hardening | `docs/release.md`, `CHANGELOG.md`, `docs/public-benchmark-table.md`, `.github/workflows/release.yml` | N/A | Versioned CLI, install docs, Docker docs, benchmark prerequisites, htslib build docs, generated benchmark table, and binary release workflow scaffolding are tracked | Release artifacts still need a real tagged GitHub run |

## Claims Not Yet Proven

- Broader performance across public real-world VCFs beyond GIAB/IGSR measured tiers.
- Performance on ten-million-record datasets and whole public cohort VCFs.
- Speed claims for BCF input, BGZF output, and compatibility TSV/stats paths.
- Additional full-file public human or plant FORMAT-rich runtime evidence beyond the bounded 50k DDBJ human tiers and measured ENA sheep/cattle FORMAT-rich expression rows.
- Threaded native BGZF evidence beyond bounded IGSR chr22 1M, including whole-cohort public rows.
- Parallel ordinary gzip, parallel native output compression, and additional full-file human/plant public FORMAT-heavy cohorts beyond ENA ERZ324584, ERZ18456468, and bounded DDBJ CHM13 chr22.
- Broader VCFtools replacement evidence beyond supported diploid biallelic normalized rows and the current staged bounded human cohort.
- Parquet export and repeated-query performance beyond the current bounded IGSR richer DuckDB workflow, including Polars/PyArrow and BCF/region Parquet workflows.
- BCF/region Parquet export through htslib compatibility paths.
- htslib-backed support for native-only `ANY`/`ALL` FORMAT aggregate predicates.
- Broader columnar Arrow/Parquet execution beyond the native selected-column first slice.

## Claim Matrix

The project ambition is to become the best practical VCF tool, but public language must stay exact: VCF-Fast only says it beats, matches, or complements competitors where the evidence supports that word.

| Claim area | bcftools/HTSlib | VCFtools | GATK | Current claim | Evidence path | Caveat |
|---|---|---|---|---|---|---|
| Streaming selective filter on supported predicates | beats on tracked synthetic, stress, GIAB, and IGSR measured tiers | later baseline | later heavier baseline | beats for measured supported native/public filter cases | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/stress-speed-benchmark.md`, `benchmark/reports/v08-core-efficiency-benchmark.md` | IGSR whole 1M heavy run still pending. |
| Core record correctness for supported filters | matches filtered core records | later baseline | later baseline | matches for supported comparisons | integration tests and benchmark equivalence diffs | Not byte-for-byte equivalent on htslib compatibility paths. |
| TSV export for selected columns | mixed: beats synthetic/stress/GIAB 1M/IGSR 10k, trails bcftools on some public and region cases | complements older workflows | complements heavier workflow exports | correctness matches; performance depends on dataset/path | `tests/convert_cli_tests.rs`, benchmark reports, `benchmark/reports/v08-core-efficiency-benchmark.md` | htslib TSV path needs optimization; Parquet is native selected-column only so far. |
| Parquet export for selected columns | beats measured stress `bcftools query` projection while producing typed columnar output | complements | complements heavier workflow exports | correctness proven for native selected-column export; measured stress projection win | `tests/convert_cli_tests.rs`, `benchmark/reports/v10-parquet-export-benchmark.md` | Native TSV is faster; public workflow benchmarks and BCF/region support are pending. |
| Export-once repeated-query workflow | beats measured repeated `bcftools view` scans for bounded IGSR row-count workflow | complements | complements heavier cohort workflows | measured DuckDB row-count workflow win after VCF-Fast Parquet export | `benchmark/reports/v10-columnar-workflow-benchmark.md` | Row-count only; richer SQL predicates, public 1M, DuckDB/Polars/PyArrow comparisons, and BCF/region Parquet are pending. |
| Public parallel plus richer columnar checks | threaded BGZF input beats measured bounded IGSR public-heavy filter rows; stress parallel native beats default and `bcftools`; richer DuckDB workflows beat repeated `bcftools` scans where correctness matched | complements | complements heavier cohort workflows | v1.2 shows public I/O-bound filtering benefits most from threaded BGZF input, while predicate parallelism helps CPU-heavy stress FORMAT aggregates; export-once DuckDB workflows win for measured `QUAL`, `INFO/DP`, `FILTER`, and grouped `CHROM,FILTER` queries | `benchmark/reports/v12-public-parallel-workflow-benchmark.md` | Requested public 1M tier reached 191526 available records; Polars/PyArrow, BCF/region Parquet, and additional public FORMAT-heavy cohorts are pending. |
| Stats simple counts | matches overlapping counts; beats stress native stats but trails bcftools on public indexed-region stats | later baseline | later baseline | matches overlapping simple counts | `tests/stats_diff_cli_tests.rs`, stress and public reports, `benchmark/reports/v08-core-efficiency-benchmark.md` | Rich `bcftools stats` parity is not claimed. |
| BCF, BGZF, tabix regions | matches ecosystem compatibility through optional htslib path, but bcftools is faster in measured synthetic compatibility runs | complements | complements | compatibility matches; speed not claimed | `tests/compatibility_cli_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | Optimize or avoid htslib reconstruction overhead. |
| Native expression parity | beats measured deterministic stress expression cases and measured public ENA sheep/cattle plus DDBJ human FORMAT breadth rows while matching filtered core records | later baseline | later baseline | supports arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and native `ANY`/`ALL` aggregate predicates; measured stress and public FORMAT expression wins | `tests/expr_tests.rs`, `tests/filter_cli_tests.rs`, `benchmark/reports/v09-expression-parity-benchmark.md`, `benchmark/reports/v18-public-format-expression-breadth.md`, `benchmark/reports/v19-second-public-format-cohort.md`, `benchmark/reports/v20-human-format-cohort.md` | htslib-backed aggregate predicates and full-file human/plant FORMAT-rich rows are pending. |
| Public FORMAT-heavy aggregate predicates | beats measured 453-sample sheep, 29-sample cattle, and bounded 3715-sample human public cohort rows and matches correctness | later baseline | later baseline | repeated ENA Ovis aries, Dutch Genebank Cattle, and DDBJ CHM13 evidence is correctness-positive and faster on measured AD, DP, GQ, selected-sample, and mixed FORMAT expression tiers | `benchmark/reports/v17-public-format-baselines.md`, `benchmark/reports/v18-public-format-expression-breadth.md`, `benchmark/reports/v19-second-public-format-cohort.md`, `benchmark/reports/v20-human-format-cohort.md` | Heavy-output mode uses core-record correctness and `/dev/null` timed output; human evidence is bounded to 50k records. |
| VCFtools-style population summaries | complements | beats measured staged public human cohort rows for supported commands | later baseline | supported diploid biallelic `freq`, `missingness`, `hardy`, `het`, `pi`, `tajima-d`, `ld`, and Weir-Cockerham `fst` match normalized VCFtools rows and are faster on the staged bounded 3715-sample human cohort | `tests/popgen_cli_tests.rs`, `make vcftools-parity`, `benchmark/reports/vcftools-popgen-parity-benchmark.md` | Three measured public runs; requested 1k/10k/50k tiers all resolved to 682 actual staged biallelic records from this cached source; HWE exact p-value is outside current output; Fst public populations are auto-derived benchmark files. |
| Threaded native BGZF input | beats measured bounded IGSR BGZF filter rows and matches filtered core records | later baseline | later baseline | default auto-capped threaded BGZF input improves the compressed native filter path through the v1.4 measured public scale rows: `2.26x` to `2.39x` faster than forced single-thread native and `13.44x` to `13.47x` faster than `bcftools`; `VCF_FAST_NATIVE_BGZF_THREADS=1` preserves a single-thread comparison path | `src/io.rs`, `benchmark/reports/v10-compressed-input-benchmark.md`, `benchmark/reports/v14-public-parallel-scale-benchmark.md` | BGZF-only; ordinary gzip remains single-thread fallback; bounded public region reached 191526 records for the requested 1M tier. |
| Query-aware `.vfi` indexed filtering | beats bcftools on measured skip-heavy synthetic rows and guarded bounded public IGSR AF rows; raw index use is reserved for high-skip plans | later baseline | later baseline | BGZF virtual-offset `.vfi` filtering preserves default-native byte output and matches bcftools core records; synthetic 1M skip-heavy filtering measured `6.06x` faster than default native and `55.80x` faster than bcftools, while bounded public IGSR `AF > 0.99` rows now use a guarded planner that falls back to default native on low-skip plans and measured `10.01x` to `13.31x` faster than bcftools | `benchmark/reports/v21-indexed-filter-benchmark.md`, `benchmark/reports/v21-public-indexed-filter-benchmark.md` | Public AF rows skipped only `0.0%` to `50.0%` of chunks; raw index acceleration still needs high-skip predicates or deeper BGZF scheduling to beat default native on public data. |
| Parallel native predicate evaluation | beats measured default-native and `bcftools` CPU-heavy stress filter rows while preserving record order | later baseline | later baseline | opt-in ordered-batch parallel evaluation helps CPU-heavy aggregate FORMAT filters | `src/engine/filter.rs`, `benchmark/reports/v11-parallel-native-filter-benchmark.md` | Synthetic stress only; public and I/O-bound cases are pending. |
| Whole public cohort performance | beats bcftools on measured GIAB/IGSR native filter tiers | later baseline | later heavy baseline | partially proven for supported filter cases | `benchmark/reports/public-whole-cohort-benchmark.md` | IGSR whole 1M deferred; broader cohorts still pending. |

## Next Contribution Targets

1. Keep v0.8 evidence current as new byte-core runs are added, and use `benchmark/reports/v08-core-efficiency-benchmark.md` as the source for README/contribution claims.
2. Add repeated/default-run VCFtools public-cohort rows at larger staged tiers and real population files, then broaden only the VCFtools workflows whose normalized parity still passes.
3. Tune auto BGZF thread caps only from full v1.4 rows; keep predicate parallelism opt-in unless larger repeated public FORMAT-heavy evidence proves a safe default.
4. Run a real tagged v1.4 release once CI is green and confirm uploaded Linux/macOS archives.
5. Complete the Bioconda release and professional rename decision checklist in `TODO.md` before publishing external installation instructions.
6. Expand v1.5 columnar workflow baselines to Polars/PyArrow and BCF/region Parquet after the measured DuckDB predicate/grouped rows.
7. Keep BCF TSV compatibility optimization as a tracked gap.
