# VCF-Fast Contribution Map

VCF-Fast is not intended to be only a faster VCF scanner. The project is building toward a benchmarked execution engine for post-calling variant operations.

## Current Implemented Contributions

### Selective Parsing

The filter engine parses only fields required by the expression. For example, `QUAL > 30` does not parse INFO values, while `INFO/DP > 40` extracts the INFO field needed for that predicate.

Evidence:

- Filter implementation uses required-field analysis from the expression AST.
- Tests cover borrowed record parsing that skips unneeded INFO parsing.
- Shared borrowed VCF field parsing avoids per-record column vectors in filter, stats, and TSV conversion paths.
- Benchmarks show speedups for QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases in the synthetic benchmark harness.

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
```

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

## Claims Supported So Far

- Correctness: VCF-Fast matches bcftools filtered core records for supported synthetic QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases.
- Performance: On the tracked 1M synthetic benchmark run, VCF-Fast was `1.62x` to `1.82x` faster than bcftools across measured supported filter cases and `1.57x` faster for TSV conversion.
- Public smoke: On the first 10k GIAB HG002 public-small run, VCF-Fast matched bcftools outputs and measured `2.08x` to `2.11x` faster for QUAL filtering and `1.12x` faster for TSV conversion.
- Public region: On the tracked IGSR chr22 100k public-region run, VCF-Fast matched bcftools outputs and measured `5.35x` to `8.33x` faster for QUAL filtering and `1.11x` faster for TSV conversion.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md` and `benchmark/reports/public-dataset-benchmark.md` | `bcftools filter` | `1.62x` to `1.82x` faster on supported 1M synthetic cases; `5.35x` to `8.33x` faster on IGSR chr22 100k QUAL public-region cases | Whole-cohort public runs still pending |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons | FORMAT predicates not supported |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked in synthetic benchmark; `1.57x` faster at 1M synthetic records | No Parquet/Arrow yet |
| Public data benchmarking | `benchmark/reports/public-dataset-benchmark.md` | `bcftools filter`, `bcftools query` | GIAB HG002 and IGSR chr22 subsets matched bcftools with runtime, throughput, and RSS reporting | Whole-cohort public runs still pending |

## Claims Not Yet Proven

- Broader performance across whole public real-world VCFs beyond GIAB/IGSR subsets.
- Performance on ten-million-record datasets and whole public cohort VCFs.
- BGZF/tabix-compatible compressed output.
- FORMAT/sample-specific filtering.
- Persistent columnar Arrow/Parquet execution.

## Next Contribution Targets

1. Larger synthetic stress datasets with many unused INFO/FORMAT fields.
2. FORMAT-aware selective parsing for sample-level predicates.
3. Evaluate `rust-htslib`/htslib interop for BGZF, BCF, tabix, and indexed reads.
4. Arrow/Parquet export for repeated analytical workloads.
