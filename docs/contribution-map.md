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

The `convert --to tsv` command exports VCF records into stable, analysis-friendly columns while preserving missing values and comma-separated INFO/AF values.

Evidence:

- Integration tests cover plain and gzip input.
- Benchmark harness compares normalized TSV rows against `bcftools query`.

### Compatibility Interop

The default engine remains Rust-native and dependency-light. With `--features htslib` or `--features htslib-static`, VCF-Fast can route compatibility-only operations through HTSlib: BCF input, tabix-indexed region reads, and BGZF output.

Evidence:

- Backend selector tests cover native vs htslib routing.
- Default-build CLI tests verify clear errors for `.bcf`, `--region`, and `--compression bgzf`.
- htslib-enabled integration tests cover BCF filtering, BCF TSV conversion, indexed region filtering, indexed BCF region stats, and BGZF output that is gzip-readable and tabix-indexable.
- Competitor design notes are tracked in `docs/competitor-notes.md`.

## Claims Supported So Far

- Correctness: VCF-Fast matches bcftools filtered core records for supported synthetic QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases.
- Performance: On the tracked 1M synthetic benchmark run, VCF-Fast was `1.62x` to `1.82x` faster than bcftools across measured supported filter cases and `1.57x` faster for TSV conversion.
- Public whole-cohort tiers: On the v0.6 Docker run, GIAB HG002 matched bcftools outputs and measured `1.80x` to `2.38x` faster for plain QUAL filtering, `1.89x` faster for 1M gzip QUAL filtering, and `1.13x` faster for 1M TSV conversion; smaller GIAB TSV/gzip tiers were mixed.
- Public IGSR tiers: On the v0.6 Docker run, IGSR chr22 10k/100k tiers matched bcftools outputs and measured `4.85x` to `5.71x` faster for QUAL filtering; TSV measured `1.22x` at 10k and `0.87x` at 100k. IGSR 1M whole-file is deferred because the generated plain VCF exceeded 13 GB during the balanced run.
- Public indexed region: On the v0.6 Docker run, IGSR chr22 indexed-region filtering matched bcftools and measured `1.47x` faster at 10k and 100k; region TSV and stats matched correctness but measured `0.71x` to `0.72x`, so bcftools is faster there.
- Public heavy sample-rich path: On the v0.7 bounded public-heavy runs after optimization, VCF-Fast matched bcftools outputs and measured `5.23x` to `5.65x` faster for gzip QUAL filtering at 100k/1M and `1.08x` to `1.10x` faster for gzip TSV conversion at 100k/1M. The bottleneck was narrowed to wide-line materialization plus gzip backend cost; the fix uses streaming TSV field reads, tail skipping after INFO, and flate2 with `zlib-ng`.
- Stress speed: On the tracked 1M synthetic stress benchmark with 40 unused INFO fields and 16 samples, VCF-Fast matched bcftools outputs and measured `1.96x` to `2.45x` faster for plain filter cases, `1.20x` faster for TSV conversion, and `1.53x` faster for overlapping stats record counts.
- FORMAT-aware filtering: On the tracked 1M synthetic stress benchmark, selected-sample `FORMAT/DP`, `FORMAT/GQ`, and `FORMAT/GT` filters matched bcftools filtered core records and measured `1.99x` to `2.06x` faster.
- v0.9 native expression parity: Fixture-backed tests and the v0.9 benchmark report cover arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>` with `--sample`, and native sample aggregate predicates such as `ANY(FORMAT/DP > 20)` and `ALL(FORMAT/GQ >= 30)`. On deterministic stress VCFs with 40 unused INFO fields and 16 samples, VCF-Fast matched `bcftools filter` core records and measured `2.66x` to `3.90x` faster at 10k records and `2.41x` to `5.18x` faster at 100k records; public v0.9 expression rows are still pending.
- Compatibility proof: Optional htslib-backed paths cover BCF input, indexed region reads, and BGZF output. v0.7 typed TSV/stats optimization matched correctness and moved several compatibility paths to near parity or faster, including 1M BCF filter `1.05x`, indexed-region filter `1.25x`, and indexed BCF stats `1.05x`; BCF TSV still trails `bcftools query` at `0.50x` for 1M.
- v0.8 byte-core evidence: On the repeated post-surgery benchmark, VCF-Fast matched supported correctness checks. The measured stress and IGSR public-heavy results are recorded in `benchmark/reports/v08-core-efficiency-benchmark.md`: stress filters were `3.14x` to `6.24x` faster, stress TSV was `2.54x` faster, stress stats were `2.50x` faster, public-heavy QUAL was `6.01x` faster, and public-heavy TSV was `1.13x` faster; all v0.8 rows were measured wins, with caveats limited to synthetic stress shape and bounded chr22 region.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, and `benchmark/reports/public-whole-cohort-benchmark.md` | `bcftools filter` | `1.62x` to `1.82x` faster on supported 1M synthetic cases; GIAB public tiers up to `2.38x`; IGSR public tiers up to `5.71x`; indexed-region filtering `1.47x` | IGSR whole 1M deferred after >13 GB intermediate |
| Stress selective parsing | `benchmark/reports/stress-speed-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | `1.96x` to `2.45x` faster on 1M plain stress filters; `1.20x` TSV speedup; `1.53x` stats speedup | Synthetic stress shape, not a public cohort |
| Selected-sample FORMAT filtering | `benchmark/reports/format-filter-benchmark.md` | `bcftools filter` | `1.99x` to `2.06x` faster on 1M selected-sample FORMAT filters | Single selected sample only; synthetic stress shape |
| v0.9 native expression parity | `tests/expr_tests.rs`, `tests/filter_cli_tests.rs`, `benchmark/reports/v09-expression-parity-benchmark.md` | `bcftools filter` | Arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and `ANY(FORMAT/<KEY>)` / `ALL(FORMAT/<KEY>)` semantics are documented, fixture-tested, and measured `2.41x` to `5.18x` faster at 100k deterministic stress records with matched core records | Synthetic stress expression evidence only; public v0.9 expression rows pending |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons, arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and native sample aggregates | htslib-backed aggregate predicates are rejected in v0.9 |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked; synthetic 1M `1.57x`; GIAB 1M `1.13x`; public-heavy 100k/1M gzip `1.08x` to `1.10x` after v0.7 optimization | Mixed public results: GIAB 10k/100k, IGSR 100k, and BCF TSV trail bcftools |
| Public data benchmarking | `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/v07-heavy-run-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | GIAB HG002, IGSR chr22, stress, and bounded public-heavy 100k/1M runs measured with correctness, runtime, throughput, and RSS reporting | Whole-cohort sample-rich public evidence beyond bounded chr22 region still pending |
| Compatibility interop | `tests/compatibility_cli_tests.rs`, `tests/compatibility_unit_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | `bcftools`, `tabix`, HTSlib | BCF input, indexed region reads, and BGZF output are feature-gated, tested, benchmarked, and near parity/faster on several v0.7 rows | BCF TSV remains slower than bcftools query |

## Claims Not Yet Proven

- Broader performance across public real-world VCFs beyond GIAB/IGSR measured tiers.
- Performance on ten-million-record datasets and whole public cohort VCFs.
- Speed claims for BCF input, BGZF output, and compatibility TSV/stats paths.
- Public real-cohort runtime evidence for v0.9 arbitrary expression parity.
- htslib-backed support for native-only `ANY`/`ALL` FORMAT aggregate predicates.
- Persistent columnar Arrow/Parquet execution.

## Claim Matrix

The project ambition is to become the best practical VCF tool, but public language must stay exact: VCF-Fast only says it beats, matches, or complements competitors where the evidence supports that word.

| Claim area | bcftools/HTSlib | VCFtools | GATK | Current claim | Evidence path | Caveat |
|---|---|---|---|---|---|---|
| Streaming selective filter on supported predicates | beats on tracked synthetic, stress, GIAB, and IGSR measured tiers | later baseline | later heavier baseline | beats for measured supported native/public filter cases | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/stress-speed-benchmark.md`, `benchmark/reports/v08-core-efficiency-benchmark.md` | IGSR whole 1M heavy run still pending. |
| Core record correctness for supported filters | matches filtered core records | later baseline | later baseline | matches for supported comparisons | integration tests and benchmark equivalence diffs | Not byte-for-byte equivalent on htslib compatibility paths. |
| TSV export for selected columns | mixed: beats synthetic/stress/GIAB 1M/IGSR 10k, trails bcftools on some public and region cases | complements older workflows | complements heavier workflow exports | correctness matches; performance depends on dataset/path | `tests/convert_cli_tests.rs`, benchmark reports, `benchmark/reports/v08-core-efficiency-benchmark.md` | Arrow/Parquet not implemented yet; htslib TSV path needs optimization. |
| Stats simple counts | matches overlapping counts; beats stress native stats but trails bcftools on public indexed-region stats | later baseline | later baseline | matches overlapping simple counts | `tests/stats_diff_cli_tests.rs`, stress and public reports, `benchmark/reports/v08-core-efficiency-benchmark.md` | Rich `bcftools stats` parity is not claimed. |
| BCF, BGZF, tabix regions | matches ecosystem compatibility through optional htslib path, but bcftools is faster in measured synthetic compatibility runs | complements | complements | compatibility matches; speed not claimed | `tests/compatibility_cli_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | Optimize or avoid htslib reconstruction overhead. |
| Native expression parity | beats measured deterministic stress expression cases and matches filtered core records for common expression forms | later baseline | later baseline | supports arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and native `ANY`/`ALL` aggregate predicates; measured deterministic stress wins | `tests/expr_tests.rs`, `tests/filter_cli_tests.rs`, `benchmark/reports/v09-expression-parity-benchmark.md` | Public benchmark rows and htslib aggregate support are pending. |
| Whole public cohort performance | beats bcftools on measured GIAB/IGSR native filter tiers | later baseline | later heavy baseline | partially proven for supported filter cases | `benchmark/reports/public-whole-cohort-benchmark.md` | IGSR whole 1M deferred; broader cohorts still pending. |

## Next Contribution Targets

1. Keep v0.8 evidence current as new byte-core runs are added, and use `benchmark/reports/v08-core-efficiency-benchmark.md` as the source for README/contribution claims.
2. Add public v0.9 expression parity benchmark rows before broadening the deterministic stress runtime claim for arbitrary `INFO/<KEY>`, selected `FORMAT/<KEY>`, or sample `ANY`/`ALL` predicates.
3. Plan v1.0 parallel BGZF and Parquet export as separate evidence-backed milestones.
4. Keep BCF TSV compatibility optimization as a tracked gap.
