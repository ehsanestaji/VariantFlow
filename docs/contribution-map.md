# VCF-Fast Contribution Map

VCF-Fast is not intended to be only a faster VCF scanner. The project is building toward a benchmarked execution engine for post-calling variant operations.

## Current Implemented Contributions

### Selective Parsing

The filter engine parses only fields required by the expression. For example, `QUAL > 30` does not parse INFO values, while `INFO/DP > 40` extracts the INFO field needed for that predicate.

Evidence:

- Filter implementation uses required-field analysis from the expression AST.
- Tests cover borrowed record parsing that skips unneeded INFO parsing.
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
- Performance: On the tracked 100k synthetic benchmark run, VCF-Fast was `1.50x` to `1.81x` faster than bcftools across measured supported filter cases and `1.29x` faster for TSV conversion.

## Competitor Scorecard

| Contribution | Evidence path | Competitor checked | Current result | Caveat |
|---|---|---|---|---|
| Selective filter execution | `benchmark/reports/synthetic-filter-benchmark.md` | `bcftools filter` | `1.50x` to `1.81x` faster on supported 100k synthetic cases | Public datasets still pending |
| Original-record preservation | `tests/filter_cli_tests.rs` | VCF validity by behavior and line preservation | Headers and passing records preserved | BGZF output not promised |
| Typed expression AST | `tests/expr_tests.rs` | N/A | `&&`, `||`, parentheses, string/numeric comparisons | FORMAT predicates not supported |
| Variant-key diff | `tests/stats_diff_cli_tests.rs` | planned `bcftools isec/query` | Shared/unique key TSV works | No normalized multiallelic decomposition |
| TSV conversion | `tests/convert_cli_tests.rs` and benchmark harness | `bcftools query` | Stable TSV rows checked in synthetic benchmark; `1.29x` faster at 100k synthetic records | No Parquet/Arrow yet |
| Public data benchmarking | `benchmark/download_public_data.sh` and `benchmark/run_benchmarks.sh` | `bcftools filter`, `bcftools query` | Sources pinned; public-small and tabix-backed public-region modes available | Full public report pending local download/run |

## Claims Not Yet Proven

- Performance on public real-world VCFs.
- Performance on million-record and ten-million-record datasets.
- BGZF/tabix-compatible compressed output.
- FORMAT/sample-specific filtering.
- Persistent columnar Arrow/Parquet execution.

## Next Contribution Targets

1. Run and publish public GIAB/IGSR benchmark reports from cached downloads.
2. Larger synthetic stress datasets with many unused INFO/FORMAT fields.
3. FORMAT-aware selective parsing for sample-level predicates.
4. Arrow/Parquet export for repeated analytical workloads.
