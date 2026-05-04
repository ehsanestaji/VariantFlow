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
- Public smoke: On the first 10k GIAB HG002 public-small run, VCF-Fast matched bcftools outputs and measured `2.08x` to `2.11x` faster for QUAL filtering and `1.12x` faster for TSV conversion.
- Public region: On the tracked IGSR chr22 100k public-region run, VCF-Fast matched bcftools outputs and measured `5.35x` to `8.33x` faster for QUAL filtering and `1.11x` faster for TSV conversion.
- Stress speed: On the tracked 1M synthetic stress benchmark with 40 unused INFO fields and 16 samples, VCF-Fast matched bcftools outputs and measured `1.96x` to `2.45x` faster for plain filter cases, `1.20x` faster for TSV conversion, and `1.53x` faster for overlapping stats record counts.
- FORMAT-aware filtering: On the tracked 1M synthetic stress benchmark, selected-sample `FORMAT/DP`, `FORMAT/GQ`, and `FORMAT/GT` filters matched bcftools filtered core records and measured `1.99x` to `2.06x` faster.
- Compatibility proof: Optional htslib-backed paths now cover BCF input, indexed region reads, and BGZF output. Performance evidence for these compatibility paths is not yet broad enough for a speed claim.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md` and `benchmark/reports/public-dataset-benchmark.md` | `bcftools filter` | `1.62x` to `1.82x` faster on supported 1M synthetic cases; `5.35x` to `8.33x` faster on IGSR chr22 100k QUAL public-region cases | Whole-cohort public runs still pending |
| Stress selective parsing | `benchmark/reports/stress-speed-benchmark.md` | `bcftools filter`, `bcftools query`, `bcftools stats` | `1.96x` to `2.45x` faster on 1M plain stress filters; `1.20x` TSV speedup; `1.53x` stats speedup | Synthetic stress shape, not a public cohort |
| Selected-sample FORMAT filtering | `benchmark/reports/format-filter-benchmark.md` | `bcftools filter` | `1.99x` to `2.06x` faster on 1M selected-sample FORMAT filters | Single selected sample only; synthetic stress shape |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons, selected-sample `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` | Multi-sample FORMAT predicates and arbitrary FORMAT keys not supported |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked in synthetic benchmark; `1.57x` faster at 1M synthetic records | No Parquet/Arrow yet |
| Public data benchmarking | `benchmark/reports/public-dataset-benchmark.md` | `bcftools filter`, `bcftools query` | GIAB HG002 and IGSR chr22 subsets matched bcftools with runtime, throughput, and RSS reporting | Whole-cohort public runs still pending |
| Compatibility interop | `tests/compatibility_cli_tests.rs`, `tests/compatibility_unit_tests.rs`, `benchmark/reports/compatibility-benchmark.md` | `bcftools`, `tabix`, HTSlib | BCF input, indexed region reads, and BGZF output are feature-gated and tested | Benchmark report is initial; no broad performance claim yet |

## Claims Not Yet Proven

- Broader performance across whole public real-world VCFs beyond GIAB/IGSR subsets.
- Performance on ten-million-record datasets and whole public cohort VCFs.
- Broad performance claims for BCF input, BGZF output, and tabix-indexed region reads.
- Multi-sample FORMAT predicates, ANY/ALL sample semantics, and arbitrary FORMAT keys.
- Persistent columnar Arrow/Parquet execution.

## Next Contribution Targets

1. Repeat stress and public-region benchmarks on a quieter dedicated runner.
2. Expand FORMAT predicates beyond one selected sample.
3. Run repeated compatibility benchmarks for BCF input, indexed region reads, and BGZF output against bcftools.
4. Arrow/Parquet export for repeated analytical workloads.
