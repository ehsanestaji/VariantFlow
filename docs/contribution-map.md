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
- Benchmarks show speedups for QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases in the synthetic benchmark harness.
- Stress benchmarks show speedups when records contain many unused INFO/FORMAT/sample fields.
- Selected-sample FORMAT filtering reads only the requested sample column for supported `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` predicates.

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
```

FORMAT predicates require `--sample <name>` and currently evaluate one selected sample.

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
- Public heavy sample-rich path: On the v0.7 bounded 10k public-heavy run after optimization, VCF-Fast matched bcftools outputs and measured `5.23x` faster for gzip QUAL filtering and `1.21x` faster for gzip TSV conversion. The bottleneck was narrowed to wide-line materialization plus gzip backend cost; the fix uses streaming TSV field reads, tail skipping after INFO, and flate2 with `zlib-ng`.
- Stress speed: On the tracked 1M synthetic stress benchmark with 40 unused INFO fields and 16 samples, VCF-Fast matched bcftools outputs and measured `1.96x` to `2.45x` faster for plain filter cases, `1.20x` faster for TSV conversion, and `1.53x` faster for overlapping stats record counts.
- FORMAT-aware filtering: On the tracked 1M synthetic stress benchmark, selected-sample `FORMAT/DP`, `FORMAT/GQ`, and `FORMAT/GT` filters matched bcftools filtered core records and measured `1.99x` to `2.06x` faster.
- Compatibility proof: Optional htslib-backed paths cover BCF input, indexed region reads, and BGZF output. v0.6 repeated compatibility benchmarks matched correctness but measured `0.36x` to `0.82x` versus bcftools, so compatibility is proven but speed is not claimed for these paths.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, and `benchmark/reports/public-whole-cohort-benchmark.md` | `bcftools filter` | `1.62x` to `1.82x` faster on supported 1M synthetic cases; GIAB public tiers up to `2.38x`; IGSR public tiers up to `5.71x`; indexed-region filtering `1.47x` | IGSR whole 1M deferred after >13 GB intermediate |
| Stress selective parsing | `benchmark/reports/stress-speed-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | `1.96x` to `2.45x` faster on 1M plain stress filters; `1.20x` TSV speedup; `1.53x` stats speedup | Synthetic stress shape, not a public cohort |
| Selected-sample FORMAT filtering | `benchmark/reports/format-filter-benchmark.md` | `bcftools filter` | `1.99x` to `2.06x` faster on 1M selected-sample FORMAT filters | Single selected sample only; synthetic stress shape |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons, selected-sample `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` | Multi-sample FORMAT predicates and arbitrary FORMAT keys not supported |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked; synthetic 1M `1.57x`; GIAB 1M `1.13x`; IGSR 10k `1.22x`; public-heavy 10k gzip `1.21x` after v0.7 optimization | Mixed public results: GIAB 10k/100k, IGSR 100k, and indexed-region TSV trail bcftools |
| Public data benchmarking | `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/v07-heavy-run-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | GIAB HG002, IGSR chr22, stress, and bounded public-heavy runs measured with correctness, runtime, throughput, and RSS reporting | Public-heavy 100k/1M tiers still need repeated local evidence |
| Compatibility interop | `tests/compatibility_cli_tests.rs`, `tests/compatibility_unit_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | `bcftools`, `tabix`, HTSlib | BCF input, indexed region reads, and BGZF output are feature-gated, tested, and benchmarked | Correct but slower than bcftools in measured compatibility cases |

## Claims Not Yet Proven

- Broader performance across public real-world VCFs beyond GIAB/IGSR measured tiers.
- Performance on ten-million-record datasets and whole public cohort VCFs.
- Speed claims for BCF input, BGZF output, and compatibility TSV/stats paths.
- Multi-sample FORMAT predicates, ANY/ALL sample semantics, and arbitrary FORMAT keys.
- Persistent columnar Arrow/Parquet execution.

## Claim Matrix

The project ambition is to become the best practical VCF tool, but public language must stay exact: VCF-Fast only says it beats, matches, or complements competitors where the evidence supports that word.

| Claim area | bcftools/HTSlib | VCFtools | GATK | Current claim | Evidence path | Caveat |
|---|---|---|---|---|---|---|
| Streaming selective filter on supported predicates | beats on tracked synthetic, stress, GIAB, and IGSR measured tiers | later baseline | later heavier baseline | beats for measured supported native/public filter cases | `benchmark/reports/synthetic-filter-benchmark.md`, `benchmark/reports/public-dataset-benchmark.md`, `benchmark/reports/public-whole-cohort-benchmark.md`, `benchmark/reports/stress-speed-benchmark.md` | IGSR whole 1M heavy run still pending. |
| Core record correctness for supported filters | matches filtered core records | later baseline | later baseline | matches for supported comparisons | integration tests and benchmark equivalence diffs | Not byte-for-byte equivalent on htslib compatibility paths. |
| TSV export for selected columns | mixed: beats synthetic/stress/GIAB 1M/IGSR 10k, trails bcftools on some public and region cases | complements older workflows | complements heavier workflow exports | correctness matches; performance depends on dataset/path | `tests/convert_cli_tests.rs`, benchmark reports | Arrow/Parquet not implemented yet; htslib TSV path needs optimization. |
| Stats simple counts | matches overlapping counts; beats stress native stats but trails bcftools on public indexed-region stats | later baseline | later baseline | matches overlapping simple counts | `tests/stats_diff_cli_tests.rs`, stress and public reports | Rich `bcftools stats` parity is not claimed. |
| BCF, BGZF, tabix regions | matches ecosystem compatibility through optional htslib path, but bcftools is faster in measured synthetic compatibility runs | complements | complements | compatibility matches; speed not claimed | `tests/compatibility_cli_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | Optimize or avoid htslib reconstruction overhead. |
| Whole public cohort performance | beats bcftools on measured GIAB/IGSR native filter tiers | later baseline | later heavy baseline | partially proven for supported filter cases | `benchmark/reports/public-whole-cohort-benchmark.md` | IGSR whole 1M deferred; broader cohorts still pending. |

## Next Contribution Targets

1. Complete v0.7 heavy-run benchmarking without giant plain IGSR intermediates and optimize HTSlib-backed TSV/stats/BCF/BGZF paths where measurements show low-risk wins.
2. Run and publish the full sample-rich IGSR heavy benchmark, expand measured tiers, and fill the v0.7 report rows with correctness, runtime, memory, bottleneck, caveat, and next-action evidence.
3. Expand FORMAT predicates beyond one selected sample.
4. Arrow/Parquet export for repeated analytical workloads.
